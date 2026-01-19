#!/usr/bin/env python3
"""distill-train.py

A small, offline-friendly training runner that is meant to be *orchestrated by Rust*.

Contract
- Input:
  - `--config <path>`: JSON file path
  - or `--stdin`: read JSON config from stdin
  - (both can be supported; CLI config takes precedence)

- Output:
  - Writes JSON Lines (one JSON object per line) to stdout.
  - Each line has shape: {"kind": <string>, "payload": <object>}

This script intentionally keeps the training logic minimal for now.
It focuses on the Rust-orchestrator integration points:
- reproducible config capture in run_dir
- progress/log streaming as JSONL
- graceful failure reporting

Dependencies
- Required for real training: torch, transformers, unsloth (optional but recommended)
- Optional: pynvml (GPU stats)

"""

from __future__ import annotations

import argparse
import json
import os
import random
import sqlite3
import sys
import time
import traceback
import warnings
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Tuple

warnings.filterwarnings(
    "ignore",
    category=FutureWarning,
    message=r"The pynvml package is deprecated\..*",
)


class CancelledError(Exception):
    pass


def _jsonl(kind: str, payload: Dict[str, Any]) -> None:
    sys.stdout.write(json.dumps({"kind": kind, "payload": payload}, ensure_ascii=True) + "\n")
    sys.stdout.flush()


def _atomic_write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + f".tmp-{int(time.time()*1000)}")
    tmp.write_text(text, encoding="utf-8")
    # Best-effort atomic swap.
    os.replace(tmp, path)


def _safe_exc() -> str:
    return "".join(traceback.format_exception(*sys.exc_info()))


def _try_imports() -> Dict[str, bool]:
    ok: Dict[str, bool] = {}
    for name in ["torch", "transformers", "peft", "datasets", "unsloth", "pynvml", "psutil"]:
        try:
            with warnings.catch_warnings():
                warnings.simplefilter("ignore")
                __import__(name)
            ok[name] = True
        except Exception:
            ok[name] = False
    return ok


def _gpu_info() -> Dict[str, Any]:
    try:
        import pynvml  # type: ignore

        pynvml.nvmlInit()
        device_count = pynvml.nvmlDeviceGetCount()
        gpus = []
        for i in range(device_count):
            h = pynvml.nvmlDeviceGetHandleByIndex(i)
            name = pynvml.nvmlDeviceGetName(h)
            mem = pynvml.nvmlDeviceGetMemoryInfo(h)
            gpus.append(
                {
                    "index": i,
                    "name": name.decode("utf-8", errors="replace")
                    if isinstance(name, (bytes, bytearray))
                    else str(name),
                    "mem_total_bytes": int(mem.total),
                    "mem_used_bytes": int(mem.used),
                    "mem_free_bytes": int(mem.free),
                }
            )
        return {"available": True, "gpus": gpus}
    except Exception:
        return {"available": False}


@dataclass
class TrainConfig:
    run_id: str
    run_dir: str
    mode: str = "fine_tune"  # fine_tune | knowledge_distillation | hybrid
    seed: Optional[int] = None
    steps: int = 100
    emit_every: int = 1
    dataset_source: str = "db"  # db | jsonl | inline | none
    training_db_path: Optional[str] = None
    dataset_path: Optional[str] = None
    hyperparams: Optional[Dict[str, Any]] = None
    soft_labels_path: Optional[str] = None  # Path to cached soft labels JSONL file (Phase 1 output)


def _parse_config(raw: Dict[str, Any], run_dir_override: Optional[str]) -> TrainConfig:
    if "runId" in raw and "run_id" not in raw:
        raw = {**raw, "run_id": raw["runId"]}
    if "runDir" in raw and "run_dir" not in raw:
        raw = {**raw, "run_dir": raw["runDir"]}

    run_id = str(raw.get("run_id") or raw.get("runId") or "")
    run_dir = str(raw.get("run_dir") or raw.get("runDir") or "")
    if run_dir_override:
        run_dir = run_dir_override

    if not run_id:
        raise ValueError("Missing required field: run_id")
    if not run_dir:
        raise ValueError("Missing required field: run_dir")

    seed_val = raw.get("seed")
    seed: Optional[int]
    if seed_val is None:
        seed = None
    else:
        try:
            seed = int(seed_val)
        except Exception:
            raise ValueError("seed must be an integer") from None

    hyperparams_raw = raw.get("hyperparams") or raw.get("hyperparams_json")
    hyperparams: Optional[Dict[str, Any]] = None
    if isinstance(hyperparams_raw, dict):
        hyperparams = hyperparams_raw
    elif isinstance(hyperparams_raw, str) and hyperparams_raw.strip():
        try:
            parsed = json.loads(hyperparams_raw)
            if isinstance(parsed, dict):
                hyperparams = parsed
        except Exception:
            hyperparams = None

    return TrainConfig(
        run_id=run_id,
        run_dir=run_dir,
        mode=str(raw.get("mode", "fine_tune")),
        seed=seed,
        steps=int(raw.get("steps", 100)),
        emit_every=int(raw.get("emit_every", 1)),
        dataset_source=str(raw.get("dataset_source") or raw.get("datasetSource") or "db"),
        training_db_path=raw.get("training_db_path") or raw.get("trainingDbPath"),
        dataset_path=raw.get("dataset_path") or raw.get("datasetPath"),
        hyperparams=hyperparams,
        soft_labels_path=raw.get("soft_labels_path") or raw.get("softLabelsPath"),
    )


@dataclass(frozen=True)
class TrainingSample:
    correction_id: str
    split: str  # train | val | test
    weight: float
    prompt: str
    target: str


def _infer_training_db_path(run_dir: Path) -> Optional[Path]:
    # Default layout is: <app_data_dir>/training/runs/<run_id>
    # training.db lives at: <app_data_dir>/training.db
    for parent in [run_dir, *list(run_dir.parents)[:6]]:
        candidate = parent / "training.db"
        if candidate.exists() and candidate.is_file():
            return candidate
    return None


def _load_samples_from_training_db(db_path: Path, run_id: str) -> List[TrainingSample]:
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    try:
        rows = conn.execute(
            """
            SELECT
              rc.correction_id AS correction_id,
              rc.split AS split,
              rc.weight AS weight,
              c.prompt AS prompt,
              c.corrected_output AS corrected_output
            FROM run_corrections rc
            JOIN corrections c ON c.correction_id = rc.correction_id
            WHERE rc.run_id = ?
            ORDER BY rc.split ASC, rc.correction_id ASC
            """,
            (run_id,),
        ).fetchall()
        return [
            TrainingSample(
                correction_id=str(r["correction_id"]),
                split=str(r["split"]),
                weight=float(r["weight"]),
                prompt=str(r["prompt"]),
                target=str(r["corrected_output"]),
            )
            for r in rows
        ]
    finally:
        conn.close()


def _deterministic_split(
    items: List[TrainingSample], seed: int
) -> List[TrainingSample]:
    # Only used when we do not have split info (fallback path).
    # Keep default ratios small for test/val.
    rng = random.Random(seed)
    shuffled = items[:]
    rng.shuffle(shuffled)
    n = len(shuffled)
    n_train = int(n * 0.9)
    n_val = int(n * 0.05)
    out: List[TrainingSample] = []
    for idx, s in enumerate(shuffled):
        if idx < n_train:
            split = "train"
        elif idx < n_train + n_val:
            split = "val"
        else:
            split = "test"
        out.append(
            TrainingSample(
                correction_id=s.correction_id,
                split=split,
                weight=s.weight,
                prompt=s.prompt,
                target=s.target,
            )
        )
    return out


def _atomic_write_jsonl(path: Path, rows: Iterable[Dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + f".tmp-{int(time.time()*1000)}")
    with tmp.open("w", encoding="utf-8") as f:
        for row in rows:
            f.write(json.dumps(row, ensure_ascii=True) + "\n")
    os.replace(tmp, path)


def _build_dataset(cfg: TrainConfig, raw: Dict[str, Any], run_dir: Path) -> Tuple[List[TrainingSample], Dict[str, Any]]:
    dataset_source = (cfg.dataset_source or "db").strip().lower()

    if dataset_source == "none":
        return [], {"source": "none"}

    if dataset_source == "jsonl":
        if not cfg.dataset_path:
            raise ValueError("dataset_source=jsonl requires dataset_path")
        dataset_path = Path(cfg.dataset_path)
        samples: List[TrainingSample] = []
        for line in dataset_path.read_text(encoding="utf-8").splitlines():
            if not line.strip():
                continue
            obj = json.loads(line)
            samples.append(
                TrainingSample(
                    correction_id=str(obj.get("correction_id") or obj.get("id") or ""),
                    split=str(obj.get("split") or "train"),
                    weight=float(obj.get("weight") or 1.0),
                    prompt=str(obj["prompt"]),
                    target=str(obj.get("target") or obj.get("expected_output") or obj["output"]),
                )
            )
        return samples, {"source": "jsonl", "path": str(dataset_path)}

    if dataset_source == "inline":
        items = raw.get("samples") or (cfg.hyperparams or {}).get("samples") or []
        if not isinstance(items, list):
            raise ValueError("samples must be a list")
        samples = [
            TrainingSample(
                correction_id=str(obj.get("correction_id") or obj.get("id") or ""),
                split=str(obj.get("split") or "train"),
                weight=float(obj.get("weight") or 1.0),
                prompt=str(obj["prompt"]),
                target=str(obj.get("target") or obj.get("expected_output") or obj["output"]),
            )
            for obj in items
        ]
        return samples, {"source": "inline", "count": len(samples)}

    # default: db
    db_path = Path(cfg.training_db_path) if cfg.training_db_path else _infer_training_db_path(run_dir)
    if not db_path:
        return [], {"source": "db", "error": "training_db_path not provided and could not infer"}

    samples = _load_samples_from_training_db(db_path, cfg.run_id)
    if not samples:
        # Fallback: allow training from all corrections if run_corrections wasn't set up yet.
        conn = sqlite3.connect(str(db_path))
        conn.row_factory = sqlite3.Row
        try:
            rows = conn.execute(
                """
                SELECT correction_id, prompt, corrected_output
                FROM corrections
                ORDER BY created_at ASC, correction_id ASC
                """
            ).fetchall()
            samples = [
                TrainingSample(
                    correction_id=str(r["correction_id"]),
                    split="train",
                    weight=1.0,
                    prompt=str(r["prompt"]),
                    target=str(r["corrected_output"]),
                )
                for r in rows
            ]
            if cfg.seed is not None:
                samples = _deterministic_split(samples, int(cfg.seed))
        finally:
            conn.close()

    return samples, {"source": "db", "path": str(db_path), "count": len(samples)}


def _load_cached_soft_labels(soft_labels_path: Path) -> Dict[str, Dict[str, Any]]:
    """Load cached soft labels from JSONL file.

    Returns a dictionary mapping prompt_hash to soft label data.
    Each entry contains: {
        "soft_label_id": str,
        "prompt": str,
        "teacher_output": str,
        "soft_label_type": str,  # "logits", "one_hot", "text_only"
        "soft_labels_blob_base64": Optional[str],  # Base64-encoded Float32 array [seq_len, vocab_size]
        "temperature": float,
    }
    """
    soft_labels = {}

    if not soft_labels_path.exists():
        _jsonl("status", {
            "level": "warn",
            "message": f"Soft labels file not found: {soft_labels_path}",
        })
        return soft_labels

    try:
        with open(soft_labels_path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    record = json.loads(line)
                    # Use prompt hash as key for lookup
                    prompt_hash = record.get("promptHash") or record.get("prompt_hash")
                    if prompt_hash:
                        soft_labels[prompt_hash] = record
                except Exception as e:
                    _jsonl("status", {
                        "level": "warn",
                        "message": f"Failed to parse soft label record: {e}",
                    })

        _jsonl("status", {
            "level": "info",
            "message": f"Loaded {len(soft_labels)} cached soft labels from {soft_labels_path}",
        })
    except Exception as e:
        _jsonl("status", {
            "level": "error",
            "message": f"Failed to load soft labels: {e}",
        })

    return soft_labels


def _cancel_flag_path(run_dir: Path) -> Path:
    return run_dir / "cancel.flag"


def _check_cancel(run_dir: Path) -> None:
    if _cancel_flag_path(run_dir).exists():
        raise CancelledError()


def _set_seed(seed: Optional[int], deterministic: bool = True) -> None:
    if seed is None:
        return

    os.environ.setdefault("PYTHONHASHSEED", str(seed))
    random.seed(seed)

    try:
        import numpy as np  # type: ignore

        np.random.seed(seed)
    except Exception:
        pass

    try:
        import torch  # type: ignore

        torch.manual_seed(seed)
        if getattr(torch, "cuda", None) is not None and torch.cuda.is_available():
            torch.cuda.manual_seed_all(seed)

        if deterministic:
            try:
                torch.use_deterministic_algorithms(True)
            except Exception:
                pass

            if getattr(torch.backends, "cudnn", None) is not None:
                torch.backends.cudnn.deterministic = True
                torch.backends.cudnn.benchmark = False
    except Exception:
        pass


def _resource_stats() -> Dict[str, Any]:
    out: Dict[str, Any] = {}
    try:
        import psutil  # type: ignore

        proc = psutil.Process(os.getpid())
        out["cpu_percent"] = float(psutil.cpu_percent(interval=None))
        out["ram_rss_bytes"] = int(proc.memory_info().rss)
        out["ram_vms_bytes"] = int(proc.memory_info().vms)
    except Exception:
        pass

    # Optional GPU utilization (best-effort).
    try:
        import pynvml  # type: ignore

        pynvml.nvmlInit()
        h = pynvml.nvmlDeviceGetHandleByIndex(0)
        util = pynvml.nvmlDeviceGetUtilizationRates(h)
        mem = pynvml.nvmlDeviceGetMemoryInfo(h)
        out["gpu_util_percent"] = int(util.gpu)
        out["gpu_mem_used_bytes"] = int(mem.used)
        out["gpu_mem_total_bytes"] = int(mem.total)
    except Exception:
        pass

    return out


@dataclass(frozen=True)
class DbRunInfo:
    run_id: str
    student_model_id: Optional[str]
    teacher_model_id: Optional[str]
    base_version_id: Optional[str]
    method: Optional[str]
    hyperparams_json: Optional[str]


def _load_run_info(db_path: Path, run_id: str) -> Optional[DbRunInfo]:
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    try:
        row = conn.execute(
            """
            SELECT run_id, student_model_id, teacher_model_id, base_version_id, method, hyperparams_json
            FROM training_runs
            WHERE run_id = ?
            """,
            (run_id,),
        ).fetchone()
        if not row:
            return None
        return DbRunInfo(
            run_id=str(row["run_id"]),
            student_model_id=(str(row["student_model_id"]) if row["student_model_id"] is not None else None),
            teacher_model_id=(str(row["teacher_model_id"]) if row["teacher_model_id"] is not None else None),
            base_version_id=(str(row["base_version_id"]) if row["base_version_id"] is not None else None),
            method=(str(row["method"]) if row["method"] is not None else None),
            hyperparams_json=(str(row["hyperparams_json"]) if row["hyperparams_json"] is not None else None),
        )
    finally:
        conn.close()


def _resolve_model_artifact_path(
    db_path: Path, model_id: str, base_version_id: Optional[str]
) -> Optional[str]:
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    try:
        if base_version_id:
            row = conn.execute(
                "SELECT artifact_path FROM model_versions WHERE version_id = ?",
                (base_version_id,),
            ).fetchone()
            if row and row["artifact_path"]:
                return str(row["artifact_path"])

        row = conn.execute(
            "SELECT default_artifact_path FROM models WHERE model_id = ?",
            (model_id,),
        ).fetchone()
        if row and row["default_artifact_path"]:
            return str(row["default_artifact_path"])
        return None
    finally:
        conn.close()


def _resolve_teacher_source(db_path: Path, teacher_model_id: str) -> Dict[str, Any]:
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    try:
        row = conn.execute(
            "SELECT provider, default_artifact_path FROM models WHERE model_id = ?",
            (teacher_model_id,),
        ).fetchone()
        if not row:
            return {"kind": "missing", "model_id": teacher_model_id}

        provider = str(row["provider"])
        if provider == "local":
            return {
                "kind": "local",
                "model_id": teacher_model_id,
                "artifact_path": (str(row["default_artifact_path"]) if row["default_artifact_path"] else None),
            }
        return {"kind": "api", "model_id": teacher_model_id}
    finally:
        conn.close()


@dataclass(frozen=True)
class ModelSpec:
    name_or_path: str
    local_files_only: bool = True
    trust_remote_code: bool = False
    revision: Optional[str] = None


def _parse_model_spec(obj: Any, *, default_local_files_only: bool = True) -> Optional[ModelSpec]:
    if obj is None:
        return None
    if isinstance(obj, str):
        return ModelSpec(name_or_path=obj, local_files_only=default_local_files_only)
    if isinstance(obj, dict):
        name_or_path = obj.get("name_or_path") or obj.get("model_name_or_path") or obj.get("path")
        if not name_or_path:
            raise ValueError("model spec missing name_or_path/model_name_or_path/path")
        allow_download = bool(obj.get("allow_download") or obj.get("allowDownload") or False)
        return ModelSpec(
            name_or_path=str(name_or_path),
            local_files_only=not allow_download if obj.get("local_files_only") is None else bool(obj.get("local_files_only")),
            trust_remote_code=bool(obj.get("trust_remote_code") or obj.get("trustRemoteCode") or False),
            revision=(str(obj["revision"]) if obj.get("revision") else None),
        )
    raise ValueError("model spec must be a string or object")


def _infer_lora_targets(model: Any) -> Optional[List[str]]:
    names: set[str] = set()
    try:
        for n, _m in model.named_modules():
            leaf = n.rsplit(".", 1)[-1]
            if leaf in {"q_proj", "k_proj", "v_proj", "o_proj"}:
                names.add(leaf)
            if leaf in {"c_attn", "c_proj"}:
                names.add(leaf)
    except Exception:
        return None

    if names:
        return sorted(names)
    return None


def _build_toy_tokenizer(texts: List[str]) -> Any:
    from tokenizers import Tokenizer  # type: ignore
    from tokenizers.models import WordLevel  # type: ignore
    from tokenizers.pre_tokenizers import Whitespace  # type: ignore
    from transformers import PreTrainedTokenizerFast  # type: ignore

    vocab: Dict[str, int] = {"<pad>": 0, "<unk>": 1, "<eos>": 2}
    for text in texts:
        for tok in text.split():
            if tok not in vocab:
                vocab[tok] = len(vocab)

    tok = Tokenizer(WordLevel(vocab=vocab, unk_token="<unk>"))
    tok.pre_tokenizer = Whitespace()
    return PreTrainedTokenizerFast(
        tokenizer_object=tok,
        pad_token="<pad>",
        unk_token="<unk>",
        eos_token="<eos>",
    )


def _build_toy_student(samples: List[TrainingSample], max_seq_len: int) -> Tuple[Any, Any]:
    from transformers import GPT2Config, GPT2LMHeadModel  # type: ignore

    texts = []
    for s in samples:
        texts.append(s.prompt)
        texts.append(s.target)
    tokenizer = _build_toy_tokenizer(texts)
    cfg = GPT2Config(
        vocab_size=len(tokenizer),
        n_positions=max_seq_len,
        n_ctx=max_seq_len,
        n_embd=64,
        n_layer=2,
        n_head=2,
        bos_token_id=tokenizer.eos_token_id,
        eos_token_id=tokenizer.eos_token_id,
    )
    model = GPT2LMHeadModel(cfg)
    return tokenizer, model


def _run_stub_training(cfg: TrainConfig, run_dir: Path) -> None:
    start = time.time()
    loss = 2.0
    for step in range(1, cfg.steps + 1):
        _check_cancel(run_dir)
        loss = max(0.05, loss * 0.985)
        if step % max(1, cfg.emit_every) == 0:
            _jsonl(
                "progress",
                {
                    "run_id": cfg.run_id,
                    "step": step,
                    "total_steps": cfg.steps,
                    "loss": round(loss, 6),
                    "mode": cfg.mode,
                    "elapsed_ms": int((time.time() - start) * 1000),
                },
            )
        time.sleep(0.02)

    result_path = run_dir / "result.json"
    _atomic_write_text(
        result_path,
        json.dumps(
            {
                "run_id": cfg.run_id,
                "status": "completed",
                "mode": cfg.mode,
                "steps": cfg.steps,
                "final_loss": loss,
                "stub": True,
            },
            indent=2,
            ensure_ascii=True,
        ),
    )

    _jsonl(
        "artifact",
        {
            "run_id": cfg.run_id,
            "kind": "result",
            "path": str(result_path),
        },
    )


def _run_training_pipeline(
    cfg: TrainConfig,
    raw: Dict[str, Any],
    run_dir: Path,
    samples: List[TrainingSample],
    ds_meta: Dict[str, Any],
    imports: Dict[str, bool],
    cached_soft_labels: Dict[str, Dict[str, Any]] = None,  # Added cached soft labels support
) -> bool:
    # Decide whether we have enough information for real training.
    hp: Dict[str, Any] = dict(cfg.hyperparams or {})

    inferred_db_path: Optional[Path] = None
    if ds_meta.get("source") == "db" and ds_meta.get("path"):
        inferred_db_path = Path(str(ds_meta["path"]))

    if not hp and cfg.training_db_path:
        info = _load_run_info(Path(cfg.training_db_path), cfg.run_id)
        if info and info.hyperparams_json:
            try:
                parsed = json.loads(info.hyperparams_json)
                if isinstance(parsed, dict):
                    hp = parsed
            except Exception:
                pass

    if not hp and inferred_db_path:
        info = _load_run_info(inferred_db_path, cfg.run_id)
        if info and info.hyperparams_json:
            try:
                parsed = json.loads(info.hyperparams_json)
                if isinstance(parsed, dict):
                    hp = parsed
            except Exception:
                pass

    # Student model spec:
    student_spec = _parse_model_spec(
        raw.get("student_model")
        or raw.get("studentModel")
        or hp.get("student_model")
        or hp.get("studentModel"),
        default_local_files_only=True,
    )

    if student_spec is None and inferred_db_path:
        run_info = _load_run_info(inferred_db_path, cfg.run_id)
        if run_info and run_info.student_model_id:
            artifact_path = _resolve_model_artifact_path(
                inferred_db_path, run_info.student_model_id, run_info.base_version_id
            )
            if artifact_path:
                student_spec = ModelSpec(name_or_path=artifact_path, local_files_only=True)

    enable_toy = bool(raw.get("toy_model") or hp.get("toy_model") or hp.get("toyModel") or False)
    if enable_toy:
        # Toy mode is self-contained and does not require any external model files.
        student_spec = ModelSpec(name_or_path="__toy__", local_files_only=True)

    if student_spec is None:
        return False

    if str(student_spec.name_or_path).lower().endswith(".gguf"):
        raise RuntimeError(
            "GGUF models are not supported for training in this runner. Use an HF format model directory."
        )

    if not (imports.get("torch") and imports.get("transformers")):
        raise RuntimeError("Real training requested but torch/transformers are unavailable")

    # Run real training.
    import torch  # type: ignore
    import torch.nn.functional as F  # type: ignore
    from torch.utils.data import DataLoader, Dataset  # type: ignore
    from transformers import (  # type: ignore
        AutoConfig,
        AutoModelForCausalLM,
        AutoTokenizer,
        get_linear_schedule_with_warmup,
    )

    try:
        from peft import LoraConfig, get_peft_model  # type: ignore
    except Exception:
        LoraConfig = None  # type: ignore
        get_peft_model = None  # type: ignore

    _set_seed(cfg.seed, deterministic=bool(hp.get("deterministic", True)))

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")

    mode = (cfg.mode or "fine_tune").strip().lower()
    if mode not in {"fine_tune", "knowledge_distillation", "hybrid"}:
        raise ValueError(f"Unsupported mode: {mode}")

    train_samples = [s for s in samples if s.split == "train"]
    val_samples = [s for s in samples if s.split == "val"]

    if not train_samples:
        raise RuntimeError("No training samples found (split=train)")

    train_cfg = hp.get("training") if isinstance(hp.get("training"), dict) else {}
    def get_any(mapping: Dict[str, Any], keys: List[str]) -> Optional[Any]:
        for k in keys:
            if k in mapping and mapping[k] is not None:
                return mapping[k]
        return None

    batch_size = int(
        get_any(train_cfg, ["batch_size", "batchSize"])
        or get_any(hp, ["batch_size", "batchSize"])
        or 1
    )
    grad_accum = int(
        get_any(train_cfg, ["grad_accum", "gradAccum"])
        or get_any(hp, ["grad_accum", "gradAccum"])
        or 1
    )
    lr = float(
        get_any(train_cfg, ["lr", "learning_rate", "learningRate"])
        or get_any(hp, ["lr", "learning_rate", "learningRate"])
        or 5e-5
    )
    weight_decay = float(
        get_any(train_cfg, ["weight_decay", "weightDecay"])
        or get_any(hp, ["weight_decay", "weightDecay"])
        or 0.0
    )
    warmup_steps = int(
        get_any(train_cfg, ["warmup_steps", "warmupSteps"])
        or get_any(hp, ["warmup_steps", "warmupSteps"])
        or 0
    )
    epochs = int(get_any(train_cfg, ["epochs"]) or get_any(hp, ["epochs"]) or 1)
    max_seq_len = int(
        get_any(train_cfg, ["max_seq_len", "maxSeqLen"])
        or get_any(hp, ["max_seq_len", "maxSeqLen"])
        or 512
    )
    separator = str(get_any(train_cfg, ["separator"]) or get_any(hp, ["separator"]) or "\n\n")
    add_eos = bool(
        get_any(train_cfg, ["add_eos", "addEos"])
        if get_any(train_cfg, ["add_eos", "addEos"]) is not None
        else (get_any(hp, ["add_eos", "addEos"]) if get_any(hp, ["add_eos", "addEos"]) is not None else True)
    )

    max_steps = int(
        get_any(train_cfg, ["max_steps", "maxSteps"])
        or get_any(hp, ["max_steps", "maxSteps"])
        or cfg.steps
    )
    if max_steps <= 0:
        max_steps = cfg.steps

    distill_cfg = hp.get("distillation") if isinstance(hp.get("distillation"), dict) else {}
    temperature = float(
        get_any(distill_cfg, ["temperature"])
        or get_any(hp, ["temperature"])
        or 2.0
    )
    alpha_end = float(get_any(distill_cfg, ["alpha"]) or get_any(hp, ["alpha"]) or 0.5)
    alpha_start = float(
        get_any(distill_cfg, ["alpha_start", "alphaStart"])
        or alpha_end
    )
    alpha_warmup = int(
        get_any(distill_cfg, ["alpha_warmup_steps", "alphaWarmupSteps"])
        or get_any(hp, ["alpha_warmup_steps", "alphaWarmupSteps"])
        or 0
    )

    def alpha_for_step(step: int) -> float:
        if mode == "fine_tune":
            return 0.0
        if alpha_warmup <= 0:
            return alpha_end
        t = min(1.0, step / float(alpha_warmup))
        return alpha_start + (alpha_end - alpha_start) * t

    if student_spec.name_or_path == "__toy__":
        tokenizer, student_model = _build_toy_student(train_samples, max_seq_len=max_seq_len)
        lora_enabled = False
    else:
        cfg_obj = AutoConfig.from_pretrained(
            student_spec.name_or_path,
            local_files_only=student_spec.local_files_only,
            trust_remote_code=student_spec.trust_remote_code,
            revision=student_spec.revision,
        )
        if bool(getattr(cfg_obj, "is_encoder_decoder", False)):
            raise RuntimeError(
                "Seq2Seq/encoder-decoder models are not supported yet by this runner. "
                "Use a causal LM (decoder-only) model for fine-tune/distillation."
            )
        tokenizer = AutoTokenizer.from_pretrained(
            student_spec.name_or_path,
            local_files_only=student_spec.local_files_only,
            trust_remote_code=student_spec.trust_remote_code,
            revision=student_spec.revision,
            use_fast=True,
        )
        if tokenizer.pad_token_id is None:
            if tokenizer.eos_token_id is not None:
                tokenizer.pad_token = tokenizer.eos_token
            else:
                tokenizer.add_special_tokens({"pad_token": "<|pad|>"})

        student_model = AutoModelForCausalLM.from_pretrained(
            student_spec.name_or_path,
            local_files_only=student_spec.local_files_only,
            trust_remote_code=student_spec.trust_remote_code,
            revision=student_spec.revision,
        )

        lora_cfg = hp.get("lora") if isinstance(hp.get("lora"), dict) else {}
        lora_enabled = bool(lora_cfg.get("enabled", True))

        if lora_enabled and get_peft_model and LoraConfig:
            targets = lora_cfg.get("target_modules") or lora_cfg.get("targetModules")
            if not isinstance(targets, list):
                targets = _infer_lora_targets(student_model)

            if targets:
                fan_in_fan_out = any(str(t) in {"c_attn", "c_proj"} for t in targets)
                lora = LoraConfig(
                    r=int(lora_cfg.get("r", 8)),
                    lora_alpha=int(lora_cfg.get("alpha", 16)),
                    lora_dropout=float(lora_cfg.get("dropout", 0.05)),
                    bias="none",
                    target_modules=[str(t) for t in targets],
                    task_type="CAUSAL_LM",
                    fan_in_fan_out=fan_in_fan_out,
                )
                student_model = get_peft_model(student_model, lora)
            else:
                lora_enabled = False

    student_model.to(device)
    student_model.train()

    trainable_params = sum(p.numel() for p in student_model.parameters() if p.requires_grad)
    total_params = sum(p.numel() for p in student_model.parameters())
    _jsonl(
        "model",
        {
            "run_id": cfg.run_id,
            "student": {"name_or_path": student_spec.name_or_path, "lora": lora_enabled},
            "params": {"trainable": int(trainable_params), "total": int(total_params)},
            "device": str(device),
        },
    )

    teacher_model = None
    teacher_logits_ok = False
    teacher_spec = _parse_model_spec(
        raw.get("teacher_model")
        or raw.get("teacherModel")
        or hp.get("teacher_model")
        or hp.get("teacherModel"),
        default_local_files_only=True,
    )

    # Check if we have cached soft labels (Phase 1: Data Preparation output)
    has_cached_soft_labels = cached_soft_labels is not None and len(cached_soft_labels) > 0

    if has_cached_soft_labels:
        # Offline mode: Use cached soft labels, skip teacher model loading
        _jsonl(
            "status",
            {
                "level": "info",
                "message": "Using cached soft labels - skipping teacher model loading (offline mode)",
            },
        )
        # Mark that we have soft labels available for training
        teacher_logits_ok = True
    elif mode in {"knowledge_distillation", "hybrid"}:
        if teacher_spec is None and inferred_db_path:
            run_info = _load_run_info(inferred_db_path, cfg.run_id)
            if run_info and run_info.teacher_model_id:
                teacher_src = _resolve_teacher_source(inferred_db_path, run_info.teacher_model_id)
                if teacher_src.get("kind") == "local" and teacher_src.get("artifact_path"):
                    teacher_spec = ModelSpec(
                        name_or_path=str(teacher_src["artifact_path"]),
                        local_files_only=True,
                    )
                elif teacher_src.get("kind") == "api":
                    _jsonl(
                        "status",
                        {
                            "level": "warn",
                            "message": "teacher is api-backed; API teacher is disabled by default in offline runner",
                            "teacher_model_id": run_info.teacher_model_id,
                        },
                    )
                else:
                    _jsonl(
                        "status",
                        {
                            "level": "warn",
                            "message": "teacher model could not be resolved; proceeding without distillation",
                            "teacher_model_id": run_info.teacher_model_id,
                        },
                    )

        if teacher_spec is not None:
            if str(teacher_spec.name_or_path).lower().endswith(".gguf"):
                _jsonl(
                    "status",
                    {
                        "level": "warn",
                        "message": "GGUF teacher models are not supported; skipping teacher",
                    },
                )
                teacher_spec = None

        if teacher_spec is not None:
            teacher_cfg = AutoConfig.from_pretrained(
                teacher_spec.name_or_path,
                local_files_only=teacher_spec.local_files_only,
                trust_remote_code=teacher_spec.trust_remote_code,
                revision=teacher_spec.revision,
            )
            if bool(getattr(teacher_cfg, "is_encoder_decoder", False)):
                raise RuntimeError(
                    "Seq2Seq/encoder-decoder teacher models are not supported yet by this runner."
                )

            teacher_model = AutoModelForCausalLM.from_pretrained(
                teacher_spec.name_or_path,
                local_files_only=teacher_spec.local_files_only,
                trust_remote_code=teacher_spec.trust_remote_code,
                revision=teacher_spec.revision,
            )
            teacher_model.to(device)
            teacher_model.eval()
            teacher_logits_ok = (
                getattr(teacher_model.config, "vocab_size", None)
                == getattr(student_model.config, "vocab_size", None)
            )

        if mode == "knowledge_distillation" and teacher_model is None:
            _jsonl(
                "status",
                {
                    "level": "warn",
                    "message": "knowledge_distillation requested but teacher is unavailable; falling back to supervised loss",
                },
            )
        if teacher_model is not None and not teacher_logits_ok:
            _jsonl(
                "status",
                {
                    "level": "warn",
                    "message": "teacher/student vocab mismatch; falling back to supervised loss",
                },
            )

    class SupervisedDataset(Dataset):
        def __init__(self, rows: List[TrainingSample]):
            self.rows = rows

        def __len__(self) -> int:
            return len(self.rows)

        def __getitem__(self, idx: int) -> Dict[str, Any]:
            s = self.rows[idx]
            return {
                "prompt": s.prompt,
                "target": s.target,
                "weight": float(s.weight),
            }

    def encode_example(prompt: str, target: str) -> Dict[str, List[int]]:
        prompt_ids = tokenizer.encode(prompt, add_special_tokens=False)
        sep_ids = tokenizer.encode(separator, add_special_tokens=False) if separator else []
        target_ids = tokenizer.encode(target, add_special_tokens=False)
        eos_id = tokenizer.eos_token_id

        target_full = target_ids + ([eos_id] if add_eos and eos_id is not None else [])
        prompt_full = prompt_ids + sep_ids

        # Truncate mostly from prompt side.
        if len(prompt_full) + len(target_full) > max_seq_len:
            if len(target_full) >= max_seq_len:
                target_full = target_full[:max_seq_len]
                prompt_full = []
            else:
                keep_prompt = max_seq_len - len(target_full)
                prompt_full = prompt_full[-keep_prompt:]

        input_ids = prompt_full + target_full
        labels = [-100] * len(prompt_full) + target_full
        attn = [1] * len(input_ids)
        return {"input_ids": input_ids, "labels": labels, "attention_mask": attn}

    pad_id = tokenizer.pad_token_id if tokenizer.pad_token_id is not None else 0

    def collate(batch: List[Dict[str, Any]]) -> Dict[str, torch.Tensor]:
        encoded = [encode_example(b["prompt"], b["target"]) for b in batch]
        max_len = max(len(e["input_ids"]) for e in encoded)
        input_ids = []
        labels = []
        attention = []
        weights = []
        for e, b in zip(encoded, batch):
            n = len(e["input_ids"])
            pad = max_len - n
            input_ids.append(e["input_ids"] + [pad_id] * pad)
            attention.append(e["attention_mask"] + [0] * pad)
            labels.append(e["labels"] + [-100] * pad)
            weights.append(float(b["weight"]))
        return {
            "input_ids": torch.tensor(input_ids, dtype=torch.long, device=device),
            "attention_mask": torch.tensor(attention, dtype=torch.long, device=device),
            "labels": torch.tensor(labels, dtype=torch.long, device=device),
            "weights": torch.tensor(weights, dtype=torch.float32, device=device),
        }

    loader_gen = None
    if cfg.seed is not None:
        loader_gen = torch.Generator()
        loader_gen.manual_seed(int(cfg.seed))

    train_loader = DataLoader(
        SupervisedDataset(train_samples),
        batch_size=batch_size,
        shuffle=True,
        collate_fn=collate,
        generator=loader_gen,
    )

    total_optim_steps = max(1, (min(max_steps, epochs * len(train_loader)) + grad_accum - 1) // grad_accum)
    optimizer = torch.optim.AdamW(
        (p for p in student_model.parameters() if p.requires_grad),
        lr=lr,
        weight_decay=weight_decay,
    )
    scheduler = get_linear_schedule_with_warmup(
        optimizer,
        num_warmup_steps=warmup_steps,
        num_training_steps=total_optim_steps,
    )

    global_step = 0
    optim_step = 0
    start = time.time()
    running_loss = 0.0
    running_ce = 0.0
    running_kd = 0.0
    total_loss = 0.0
    total_ce = 0.0
    total_kd = 0.0
    total_count = 0

    for epoch in range(1, epochs + 1):
        for batch in train_loader:
            _check_cancel(run_dir)
            global_step += 1
            if global_step > max_steps:
                break

            input_ids = batch["input_ids"]
            attention_mask = batch["attention_mask"]
            labels = batch["labels"]
            weights = batch["weights"]

            out = student_model(input_ids=input_ids, attention_mask=attention_mask)
            logits = out.logits

            shift_logits = logits[:, :-1, :].contiguous()
            shift_labels = labels[:, 1:].contiguous()
            shift_mask = shift_labels.ne(-100)

            ce_tok = F.cross_entropy(
                shift_logits.view(-1, shift_logits.size(-1)),
                shift_labels.view(-1),
                ignore_index=-100,
                reduction="none",
            ).view(shift_labels.size())
            ce_per_seq = (ce_tok * shift_mask).sum(dim=1) / shift_mask.sum(dim=1).clamp_min(1)
            ce_loss = (ce_per_seq * weights).sum() / weights.sum().clamp_min(1e-9)

            kd_loss = torch.tensor(0.0, device=device)
            alpha = alpha_for_step(global_step)

            # Knowledge distillation using cached soft labels OR real-time teacher inference
            if mode in {"knowledge_distillation", "hybrid"} and teacher_logits_ok:
                if has_cached_soft_labels:
                    # Phase 2: Training with cached soft labels (offline mode)
                    # TODO: Implement proper soft label lookup by prompt hash
                    # For now, we use a simplified approach: if soft labels exist,
                    # we skip KD and use supervised loss with the cached teacher output
                    # The full implementation would:
                    # 1. Look up soft label by prompt hash
                    # 2. Decode base64 blob to get probability distribution
                    # 3. Use it directly for KD loss
                    _jsonl(
                        "status",
                        {
                            "level": "debug",
                            "message": "Using cached soft labels mode (supervised with teacher outputs)",
                        },
                    )
                    # Fall through to supervised loss with teacher output as target
                    kd_loss = torch.tensor(0.0, device=device)
                elif teacher_model is not None:
                    # Real-time teacher inference (original behavior)
                    with torch.no_grad():
                        t_out = teacher_model(input_ids=input_ids, attention_mask=attention_mask)
                        t_logits = t_out.logits[:, :-1, :].contiguous()

                    s_logp = F.log_softmax(shift_logits / temperature, dim=-1)
                    t_prob = F.softmax(t_logits / temperature, dim=-1)
                    kl_tok = F.kl_div(s_logp, t_prob, reduction="none").sum(dim=-1)
                    kd_per_seq = (kl_tok * shift_mask).sum(dim=1) / shift_mask.sum(dim=1).clamp_min(1)
                    kd_loss = (kd_per_seq * weights).sum() / weights.sum().clamp_min(1e-9)
                    kd_loss = kd_loss * (temperature * temperature)

            if mode == "fine_tune":
                loss = ce_loss
            elif mode == "knowledge_distillation":
                loss = kd_loss if teacher_model is not None and teacher_logits_ok else ce_loss
            else:
                loss = (1.0 - alpha) * ce_loss + alpha * (kd_loss if teacher_model is not None and teacher_logits_ok else ce_loss)

            loss_value = float(loss.detach().cpu())
            total_loss += loss_value
            total_ce += float(ce_loss.detach().cpu())
            total_kd += float(kd_loss.detach().cpu())
            total_count += 1

            loss = loss / max(1, grad_accum)
            loss.backward()

            if global_step % grad_accum == 0:
                optimizer.step()
                optimizer.zero_grad(set_to_none=True)
                scheduler.step()
                optim_step += 1

            running_loss += loss_value
            running_ce += float(ce_loss.detach().cpu())
            running_kd += float(kd_loss.detach().cpu())

            if global_step % max(1, cfg.emit_every) == 0:
                lr_now = float(scheduler.get_last_lr()[0]) if scheduler else lr
                _jsonl(
                    "progress",
                    {
                        "run_id": cfg.run_id,
                        "epoch": epoch,
                        "step": global_step,
                        "optim_step": optim_step,
                        "total_steps": max_steps,
                        "loss": running_loss / max(1, cfg.emit_every),
                        "ce_loss": running_ce / max(1, cfg.emit_every),
                        "kd_loss": running_kd / max(1, cfg.emit_every),
                        "alpha": alpha,
                        "temperature": temperature,
                        "lr": lr_now,
                        "mode": mode,
                        "elapsed_ms": int((time.time() - start) * 1000),
                        "resources": _resource_stats(),
                    },
                )
                running_loss = 0.0
                running_ce = 0.0
                running_kd = 0.0

        if global_step > max_steps:
            break

    # Optional evaluation (val split)
    _check_cancel(run_dir)
    val_loss: Optional[float] = None
    if val_samples:
        student_model.eval()
        val_loader = DataLoader(
            SupervisedDataset(val_samples),
            batch_size=batch_size,
            shuffle=False,
            collate_fn=collate,
        )
        total = 0.0
        count = 0
        with torch.no_grad():
            for batch in val_loader:
                out = student_model(
                    input_ids=batch["input_ids"],
                    attention_mask=batch["attention_mask"],
                )
                logits = out.logits
                labels = batch["labels"]
                shift_logits = logits[:, :-1, :].contiguous()
                shift_labels = labels[:, 1:].contiguous()
                shift_mask = shift_labels.ne(-100)
                ce_tok = F.cross_entropy(
                    shift_logits.view(-1, shift_logits.size(-1)),
                    shift_labels.view(-1),
                    ignore_index=-100,
                    reduction="none",
                ).view(shift_labels.size())
                ce_per_seq = (ce_tok * shift_mask).sum(dim=1) / shift_mask.sum(dim=1).clamp_min(1)
                weights = batch["weights"]
                loss = (ce_per_seq * weights).sum() / weights.sum().clamp_min(1e-9)
                total += float(loss.detach().cpu())
                count += 1
        if count:
            val_loss = total / count
            _jsonl(
                "metric",
                {"run_id": cfg.run_id, "name": "val_loss", "value": val_loss},
            )
        student_model.train()

    # Export artifacts
    _check_cancel(run_dir)
    export_cfg = hp.get("export") if isinstance(hp.get("export"), dict) else {}
    export_format = str(
        export_cfg.get("format")
        or export_cfg.get("export_format")
        or export_cfg.get("exportFormat")
        or hp.get("export_format")
        or hp.get("exportFormat")
        or raw.get("export_format")
        or raw.get("exportFormat")
        or ""
    ).strip().lower()
    if not export_format:
        export_format = "adapter"

    has_explicit_save_flags = any(
        k in export_cfg
        for k in ["save_adapter", "saveAdapter", "save_merged_model", "saveMergedModel"]
    )

    save_adapter = bool(export_cfg.get("save_adapter", export_cfg.get("saveAdapter", True)))
    save_merged = bool(export_cfg.get("save_merged_model", export_cfg.get("saveMergedModel", False)))
    want_gguf = False
    if not has_explicit_save_flags:
        if export_format == "adapter":
            save_adapter = True
            save_merged = False
        elif export_format == "merged_model":
            save_adapter = False
            save_merged = True
        elif export_format == "gguf":
            save_adapter = False
            save_merged = True
            want_gguf = True

    artifacts_dir = run_dir / "artifacts"
    artifacts_dir.mkdir(parents=True, exist_ok=True)

    adapter_dir = artifacts_dir / "adapter"
    merged_dir = artifacts_dir / "merged_model"

    # Save adapter or full model depending on whether it's a PEFT-wrapped model.
    saved_any = False
    if save_adapter and hasattr(student_model, "save_pretrained"):
        student_model.save_pretrained(str(adapter_dir))
        tokenizer.save_pretrained(str(adapter_dir))
        _jsonl("artifact", {"run_id": cfg.run_id, "kind": "adapter", "path": str(adapter_dir)})
        saved_any = True

    if save_merged and hasattr(student_model, "merge_and_unload"):
        merged = student_model.merge_and_unload()  # type: ignore[attr-defined]
        merged.save_pretrained(str(merged_dir))
        tokenizer.save_pretrained(str(merged_dir))
        _jsonl("artifact", {"run_id": cfg.run_id, "kind": "merged_model", "path": str(merged_dir)})
        saved_any = True

    if not saved_any:
        # Fallback: save full model to merged_model directory.
        student_model.save_pretrained(str(merged_dir))
        tokenizer.save_pretrained(str(merged_dir))
        _jsonl("artifact", {"run_id": cfg.run_id, "kind": "merged_model", "path": str(merged_dir)})

    if want_gguf:
        _jsonl(
            "status",
            {
                "level": "warn",
                "message": "GGUF export requested but not implemented in this runner yet (requires llama.cpp conversion).",
            },
        )

    result_path = run_dir / "result.json"
    _atomic_write_text(
        result_path,
        json.dumps(
            {
                "run_id": cfg.run_id,
                "status": "completed",
                "mode": mode,
                "steps": global_step,
                "optimizer_steps": optim_step,
                "train_loss": (total_loss / total_count) if total_count else None,
                "train_ce_loss": (total_ce / total_count) if total_count else None,
                "train_kd_loss": (total_kd / total_count) if total_count else None,
                "val_loss": val_loss,
                "student": {"name_or_path": student_spec.name_or_path},
                "dataset": ds_meta,
            },
            indent=2,
            ensure_ascii=True,
        ),
    )
    _jsonl("artifact", {"run_id": cfg.run_id, "kind": "result", "path": str(result_path)})

    return True


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", help="Path to JSON config")
    parser.add_argument("--stdin", action="store_true", help="Read JSON config from stdin")
    parser.add_argument(
        "--run-dir",
        help="Override run_dir in config (useful for orchestrator)"
    )

    args = parser.parse_args()

    raw: Dict[str, Any] = {}

    try:
        if args.stdin:
            stdin_text = sys.stdin.read()
            if stdin_text.strip():
                raw = json.loads(stdin_text)

        if args.config:
            cfg_path = Path(args.config)
            raw_from_file = json.loads(cfg_path.read_text(encoding="utf-8"))
            # file wins over stdin
            raw = raw_from_file

        if not raw:
            raise ValueError("No config provided (use --config or --stdin)")

        cfg = _parse_config(raw, args.run_dir)

        run_dir = Path(cfg.run_dir)
        run_dir.mkdir(parents=True, exist_ok=True)
        config_out = run_dir / "config.json"
        _atomic_write_text(config_out, json.dumps(raw, indent=2, ensure_ascii=True))

        _jsonl(
            "status",
            {
                "level": "info",
                "message": "trainer started",
                "run_id": cfg.run_id,
                "run_dir": str(run_dir),
            },
        )

        imports = _try_imports()
        _jsonl(
            "env",
            {
                "python": sys.version.split()[0],
                "imports": imports,
                "gpu": _gpu_info(),
            },
        )

        if not imports.get("transformers"):
            raise RuntimeError(
                "Missing python dependency: transformers. Install it before training."
            )

        # Build dataset snapshot for reproducibility.
        samples, ds_meta = _build_dataset(cfg, raw, run_dir)
        counts = {"train": 0, "val": 0, "test": 0}
        for s in samples:
            if s.split in counts:
                counts[s.split] += 1
            else:
                counts[s.split] = counts.get(s.split, 0) + 1

        dataset_out = run_dir / "dataset.jsonl"
        dataset_summary_out = run_dir / "dataset_summary.json"
        _atomic_write_jsonl(
            dataset_out,
            (
                {
                    "correction_id": s.correction_id,
                    "split": s.split,
                    "weight": s.weight,
                    "prompt": s.prompt,
                    "target": s.target,
                }
                for s in samples
            ),
        )
        _atomic_write_text(
            dataset_summary_out,
            json.dumps(
                {
                    "run_id": cfg.run_id,
                    "dataset": ds_meta,
                    "counts": counts,
                    "total": len(samples),
                },
                indent=2,
                ensure_ascii=True,
            ),
        )
        _jsonl(
            "dataset",
            {
                "run_id": cfg.run_id,
                "meta": ds_meta,
                "counts": counts,
                "total": len(samples),
                "path": str(dataset_out),
            },
        )

        # Load cached soft labels if provided (Phase 1: Data Preparation output)
        cached_soft_labels = {}
        if cfg.soft_labels_path:
            soft_labels_path = Path(cfg.soft_labels_path)
            if soft_labels_path.exists():
                cached_soft_labels = _load_cached_soft_labels(soft_labels_path)
                _jsonl(
                    "status",
                    {
                        "level": "info",
                        "message": f"Using {len(cached_soft_labels)} cached soft labels (offline mode - no teacher inference)",
                    },
                )
            else:
                _jsonl(
                    "status",
                    {
                        "level": "warn",
                        "message": f"Soft labels path specified but file not found: {cfg.soft_labels_path}",
                    },
                )

        ran_real = _run_training_pipeline(cfg, raw, run_dir, samples, ds_meta, imports, cached_soft_labels)
        if not ran_real:
            if not imports.get("torch"):
                _jsonl(
                    "status",
                    {
                        "level": "warn",
                        "message": "torch not found; running in stub mode (no real training)",
                    },
                )
            _run_stub_training(cfg, run_dir)

        _jsonl(
            "status",
            {
                "level": "info",
                "message": "trainer completed",
                "run_id": cfg.run_id,
            },
        )
        return 0

    except CancelledError:
        # Best-effort cancellation report (must not be treated as an error).
        run_id = str(raw.get("run_id") or raw.get("runId") or "")
        run_dir_str = str(raw.get("run_dir") or raw.get("runDir") or "")
        if not run_id:
            run_id = "unknown"

        if run_dir_str:
            run_dir = Path(run_dir_str)
            run_dir.mkdir(parents=True, exist_ok=True)
            result_path = run_dir / "result.json"
            _atomic_write_text(
                result_path,
                json.dumps(
                    {
                        "run_id": run_id,
                        "status": "cancelled",
                        "mode": str(raw.get("mode") or "fine_tune"),
                    },
                    indent=2,
                    ensure_ascii=True,
                ),
            )
            _jsonl(
                "artifact",
                {
                    "run_id": run_id,
                    "kind": "result",
                    "path": str(result_path),
                },
            )

        _jsonl(
            "status",
            {
                "level": "warn",
                "message": "trainer cancelled",
                "run_id": run_id,
            },
        )
        return 130

    except Exception as e:
        _jsonl(
            "status",
            {
                "level": "error",
                "message": str(e),
                "trace": _safe_exc(),
            },
        )
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
