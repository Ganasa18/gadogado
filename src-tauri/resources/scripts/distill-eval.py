#!/usr/bin/env python3
"""distill-eval.py

Offline evaluator for distillation models (HF/PEFT).

Contract
- Input:
  - `--config <path>`: JSON file path
  - or `--stdin`: read JSON config from stdin
- Output:
  - JSON Lines to stdout: {"kind": <string>, "payload": <object>}
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


def _jsonl(kind: str, payload: Dict[str, Any]) -> None:
    sys.stdout.write(json.dumps({"kind": kind, "payload": payload}, ensure_ascii=True) + "\n")
    sys.stdout.flush()


def _atomic_write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + f".tmp-{int(time.time()*1000)}")
    tmp.write_text(text, encoding="utf-8")
    os.replace(tmp, path)


def _atomic_write_jsonl(path: Path, rows: Iterable[Dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + f".tmp-{int(time.time()*1000)}")
    with tmp.open("w", encoding="utf-8") as f:
        for row in rows:
            f.write(json.dumps(row, ensure_ascii=True) + "\n")
    os.replace(tmp, path)


def _safe_exc() -> str:
    return "".join(traceback.format_exception(*sys.exc_info()))


def _try_imports() -> Dict[str, bool]:
    ok: Dict[str, bool] = {}
    for name in ["torch", "transformers", "peft", "pynvml", "psutil"]:
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
class EvalConfig:
    eval_id: str
    version_id: str
    dataset_id: str
    run_dir: str
    training_db_path: Optional[str] = None
    max_samples: Optional[int] = None
    max_new_tokens: int = 128
    temperature: float = 0.0
    top_p: float = 1.0
    seed: Optional[int] = None
    compute_teacher_agreement: bool = False


def _parse_config(raw: Dict[str, Any], run_dir_override: Optional[str]) -> EvalConfig:
    if "evalId" in raw and "eval_id" not in raw:
        raw = {**raw, "eval_id": raw["evalId"]}
    if "versionId" in raw and "version_id" not in raw:
        raw = {**raw, "version_id": raw["versionId"]}
    if "datasetId" in raw and "dataset_id" not in raw:
        raw = {**raw, "dataset_id": raw["datasetId"]}
    if "runDir" in raw and "run_dir" not in raw:
        raw = {**raw, "run_dir": raw["runDir"]}

    eval_id = str(raw.get("eval_id") or "")
    version_id = str(raw.get("version_id") or "")
    dataset_id = str(raw.get("dataset_id") or "")
    run_dir = str(raw.get("run_dir") or "")
    if run_dir_override:
        run_dir = run_dir_override

    if not eval_id or not version_id or not dataset_id or not run_dir:
        raise ValueError("eval_id, version_id, dataset_id, run_dir are required")

    max_new_tokens = int(raw.get("max_new_tokens") or raw.get("maxNewTokens") or 128)
    temperature = float(raw.get("temperature") or 0.0)
    top_p = float(raw.get("top_p") or raw.get("topP") or 1.0)

    seed_val = raw.get("seed")
    seed: Optional[int] = None
    if seed_val is not None:
        try:
            seed = int(seed_val)
        except Exception:
            raise ValueError("seed must be an integer") from None

    max_samples_val = raw.get("max_samples") or raw.get("maxSamples")
    max_samples: Optional[int] = None
    if max_samples_val is not None:
        max_samples = int(max_samples_val)

    return EvalConfig(
        eval_id=eval_id,
        version_id=version_id,
        dataset_id=dataset_id,
        run_dir=run_dir,
        training_db_path=raw.get("training_db_path") or raw.get("trainingDbPath"),
        max_samples=max_samples,
        max_new_tokens=max_new_tokens,
        temperature=temperature,
        top_p=top_p,
        seed=seed,
        compute_teacher_agreement=bool(
            raw.get("compute_teacher_agreement") or raw.get("computeTeacherAgreement") or False
        ),
    )


@dataclass(frozen=True)
class EvalSample:
    prompt: str
    expected: str
    metadata: Optional[Dict[str, Any]]


@dataclass(frozen=True)
class DbRunInfo:
    run_id: str
    student_model_id: Optional[str]
    teacher_model_id: Optional[str]
    base_version_id: Optional[str]


@dataclass(frozen=True)
class DbVersionInfo:
    version_id: str
    model_id: str
    run_id: Optional[str]
    artifact_path: str
    parent_version_id: Optional[str]


def _infer_training_db_path(run_dir: Path) -> Optional[Path]:
    for parent in [run_dir, *list(run_dir.parents)[:6]]:
        candidate = parent / "training.db"
        if candidate.exists() and candidate.is_file():
            return candidate
    return None


def _load_version_info(db_path: Path, version_id: str) -> DbVersionInfo:
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    try:
        row = conn.execute(
            """
            SELECT version_id, model_id, run_id, artifact_path, parent_version_id
            FROM model_versions
            WHERE version_id = ?
            """,
            (version_id,),
        ).fetchone()
        if not row:
            raise ValueError(f"Model version not found: {version_id}")
        return DbVersionInfo(
            version_id=str(row["version_id"]),
            model_id=str(row["model_id"]),
            run_id=(str(row["run_id"]) if row["run_id"] is not None else None),
            artifact_path=str(row["artifact_path"]),
            parent_version_id=(
                str(row["parent_version_id"]) if row["parent_version_id"] is not None else None
            ),
        )
    finally:
        conn.close()


def _load_run_info(db_path: Path, run_id: str) -> Optional[DbRunInfo]:
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    try:
        row = conn.execute(
            """
            SELECT run_id, student_model_id, teacher_model_id, base_version_id
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


def _load_dataset_items(db_path: Path, dataset_id: str) -> List[EvalSample]:
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    try:
        rows = conn.execute(
            """
            SELECT prompt, expected_output, metadata_json
            FROM dataset_items
            WHERE dataset_id = ?
            ORDER BY created_at ASC, item_id ASC
            """,
            (dataset_id,),
        ).fetchall()
        samples: List[EvalSample] = []
        for r in rows:
            prompt = str(r["prompt"] or "")
            expected = str(r["expected_output"] or "")
            meta_raw = r["metadata_json"]
            meta: Optional[Dict[str, Any]] = None
            if meta_raw:
                try:
                    parsed = json.loads(meta_raw)
                    if isinstance(parsed, dict):
                        meta = parsed
                except Exception:
                    meta = None
            if prompt and expected:
                samples.append(EvalSample(prompt=prompt, expected=expected, metadata=meta))
        return samples
    finally:
        conn.close()


def _is_adapter_dir(path: Path) -> bool:
    return (path / "adapter_config.json").exists() or (path / "adapter_model.safetensors").exists()


def _is_gguf(path: Path) -> bool:
    return path.is_file() and path.suffix.lower() == ".gguf"


def _normalize_text(text: str) -> str:
    return " ".join(text.strip().lower().split())


def _tokenize(text: str) -> List[str]:
    """Simple word tokenization for metric computation."""
    return text.strip().lower().split()


def _compute_bleu(prediction: str, reference: str) -> float:
    """
    Compute sentence-level BLEU score (BLEU-4 with smoothing).

    Uses a simple n-gram based approach without external dependencies.
    Falls back to nltk if available for better accuracy.
    """
    pred_tokens = _tokenize(prediction)
    ref_tokens = _tokenize(reference)

    if not pred_tokens or not ref_tokens:
        return 0.0

    # Try using nltk for better BLEU computation
    try:
        from nltk.translate.bleu_score import sentence_bleu, SmoothingFunction  # type: ignore
        smoothing = SmoothingFunction().method1
        # Use weights for BLEU-4
        return sentence_bleu([ref_tokens], pred_tokens, smoothing_function=smoothing)
    except ImportError:
        pass

    # Fallback: simple n-gram based BLEU approximation
    from collections import Counter
    import math

    def ngram_counts(tokens: List[str], n: int) -> Counter:
        return Counter(tuple(tokens[i:i+n]) for i in range(len(tokens) - n + 1))

    # Calculate n-gram precisions for n=1,2,3,4
    precisions = []
    for n in range(1, 5):
        pred_ngrams = ngram_counts(pred_tokens, n)
        ref_ngrams = ngram_counts(ref_tokens, n)

        if not pred_ngrams:
            precisions.append(0.0)
            continue

        clipped = sum(min(pred_ngrams[ng], ref_ngrams.get(ng, 0)) for ng in pred_ngrams)
        total = sum(pred_ngrams.values())

        # Smoothing: add 1 to avoid zero
        precisions.append((clipped + 1) / (total + 1))

    if all(p == 0 for p in precisions):
        return 0.0

    # Geometric mean of precisions
    log_precisions = [math.log(p) if p > 0 else float('-inf') for p in precisions]
    avg_log_precision = sum(log_precisions) / 4

    # Brevity penalty
    bp = 1.0
    if len(pred_tokens) < len(ref_tokens):
        bp = math.exp(1 - len(ref_tokens) / max(len(pred_tokens), 1))

    return bp * math.exp(avg_log_precision) if avg_log_precision > float('-inf') else 0.0


def _compute_f1(prediction: str, reference: str) -> float:
    """
    Compute token-level F1 score between prediction and reference.

    This measures the overlap between the predicted and reference tokens,
    useful for evaluating partial correctness.
    """
    pred_tokens = set(_tokenize(prediction))
    ref_tokens = set(_tokenize(reference))

    if not pred_tokens or not ref_tokens:
        return 0.0

    common = pred_tokens & ref_tokens

    if not common:
        return 0.0

    precision = len(common) / len(pred_tokens)
    recall = len(common) / len(ref_tokens)

    return 2 * precision * recall / (precision + recall)


def _compute_faithfulness(prediction: str, keywords: List[str]) -> float:
    """
    Compute faithfulness score based on keyword coverage.

    Measures what fraction of required keywords appear in the prediction.
    """
    if not keywords:
        return 1.0  # No keywords to check

    pred_lower = prediction.lower()
    hits = sum(1 for kw in keywords if kw.lower() in pred_lower)
    return hits / len(keywords)


def _strip_prompt(prompt: str, output: str) -> str:
    if output.startswith(prompt):
        return output[len(prompt):].strip()
    return output.strip()


def _set_seed(seed: Optional[int]) -> None:
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
    except Exception:
        pass


def _load_student_and_teacher(
    cfg: EvalConfig, db_path: Path, version: DbVersionInfo
) -> Tuple[Any, Any, Optional[Any]]:
    import torch  # type: ignore
    from transformers import AutoModelForCausalLM, AutoTokenizer  # type: ignore

    try:
        from peft import PeftModel  # type: ignore
    except Exception:
        PeftModel = None  # type: ignore

    artifact_path = Path(version.artifact_path)
    if _is_gguf(artifact_path):
        raise RuntimeError("GGUF artifacts are not supported for evaluation in this runner.")

    run_info = _load_run_info(db_path, version.run_id) if version.run_id else None
    base_model_path: Optional[str] = None
    if run_info and run_info.student_model_id:
        base_model_path = _resolve_model_artifact_path(
            db_path, run_info.student_model_id, run_info.base_version_id
        )

    if artifact_path.is_dir() and _is_adapter_dir(artifact_path):
        if not base_model_path:
            raise RuntimeError("Adapter artifact requires a base model path.")
        if PeftModel is None:
            raise RuntimeError("peft is required to load adapter artifacts.")
        base_model = AutoModelForCausalLM.from_pretrained(
            base_model_path,
            local_files_only=True,
        )
        tokenizer = AutoTokenizer.from_pretrained(
            base_model_path,
            local_files_only=True,
            use_fast=True,
        )
        model = PeftModel.from_pretrained(base_model, str(artifact_path))
    else:
        tokenizer = AutoTokenizer.from_pretrained(
            str(artifact_path),
            local_files_only=True,
            use_fast=True,
        )
        model = AutoModelForCausalLM.from_pretrained(
            str(artifact_path),
            local_files_only=True,
        )

    if tokenizer.pad_token_id is None and tokenizer.eos_token_id is not None:
        tokenizer.pad_token_id = tokenizer.eos_token_id

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    model.to(device)
    model.eval()

    teacher_model = None
    if cfg.compute_teacher_agreement and run_info and run_info.teacher_model_id:
        teacher_src = _resolve_teacher_source(db_path, run_info.teacher_model_id)
        if teacher_src.get("kind") == "local" and teacher_src.get("artifact_path"):
            teacher_path = str(teacher_src["artifact_path"])
            if not _is_gguf(Path(teacher_path)):
                teacher_model = AutoModelForCausalLM.from_pretrained(
                    teacher_path,
                    local_files_only=True,
                )
                teacher_model.to(device)
                teacher_model.eval()
        elif teacher_src.get("kind") == "api":
            _jsonl(
                "status",
                {
                    "level": "warn",
                    "message": "teacher is api-backed; teacher agreement disabled",
                    "teacher_model_id": run_info.teacher_model_id,
                },
            )

    return tokenizer, model, teacher_model


def _generate(
    tokenizer: Any,
    model: Any,
    prompt: str,
    max_new_tokens: int,
    temperature: float,
    top_p: float,
) -> str:
    import torch  # type: ignore

    inputs = tokenizer(prompt, return_tensors="pt", truncation=True)
    inputs = {k: v.to(model.device) for k, v in inputs.items()}

    do_sample = temperature > 0
    gen_kwargs: Dict[str, Any] = {
        "max_new_tokens": max_new_tokens,
        "do_sample": do_sample,
        "pad_token_id": tokenizer.pad_token_id,
        "eos_token_id": tokenizer.eos_token_id,
    }
    if do_sample:
        gen_kwargs["temperature"] = temperature
        gen_kwargs["top_p"] = top_p

    with torch.no_grad():
        output_ids = model.generate(**inputs, **gen_kwargs)
    decoded = tokenizer.decode(output_ids[0], skip_special_tokens=True)
    return _strip_prompt(prompt, decoded)


def _evaluate(
    cfg: EvalConfig, raw: Dict[str, Any], run_dir: Path, samples: List[EvalSample]
) -> None:
    imports = _try_imports()
    if not imports.get("transformers") or not imports.get("torch"):
        raise RuntimeError("Missing python dependencies: torch/transformers")

    _set_seed(cfg.seed)

    db_path = Path(cfg.training_db_path) if cfg.training_db_path else _infer_training_db_path(run_dir)
    if not db_path:
        raise RuntimeError("training_db_path not provided and could not infer")

    version = _load_version_info(db_path, cfg.version_id)
    tokenizer, model, teacher_model = _load_student_and_teacher(cfg, db_path, version)

    if cfg.max_samples is not None and len(samples) > cfg.max_samples:
        rng = random.Random(cfg.seed or 0)
        samples = samples[:]
        rng.shuffle(samples)
        samples = samples[: cfg.max_samples]

    total = len(samples)
    if total == 0:
        raise RuntimeError("No evaluation samples found")

    # Metric accumulators
    exact_matches = 0
    fuzzy_total = 0.0
    bleu_total = 0.0
    f1_total = 0.0
    coverage_total = 0.0
    coverage_count = 0
    faithfulness_total = 0.0
    faithfulness_count = 0
    teacher_agree = 0

    predictions: List[Dict[str, Any]] = []

    for idx, sample in enumerate(samples, start=1):
        pred = _generate(
            tokenizer,
            model,
            sample.prompt,
            cfg.max_new_tokens,
            cfg.temperature,
            cfg.top_p,
        )
        pred_norm = _normalize_text(pred)
        exp_norm = _normalize_text(sample.expected)

        # Exact match
        exact = 1 if pred_norm == exp_norm else 0
        exact_matches += exact

        # Fuzzy match (sequence matcher)
        from difflib import SequenceMatcher
        fuzzy = SequenceMatcher(None, pred_norm, exp_norm).ratio()
        fuzzy_total += fuzzy

        # BLEU score
        bleu = _compute_bleu(pred, sample.expected)
        bleu_total += bleu

        # F1 score (token overlap)
        f1 = _compute_f1(pred, sample.expected)
        f1_total += f1

        # Citation coverage (if metadata has citations)
        coverage = None
        if sample.metadata and isinstance(sample.metadata.get("citations"), list):
            citations = [str(c) for c in sample.metadata.get("citations") or []]
            if citations:
                hits = sum(1 for c in citations if c.lower() in pred_norm)
                coverage = hits / max(1, len(citations))
                coverage_total += coverage
                coverage_count += 1

        # Faithfulness (if metadata has keywords)
        faithfulness = None
        if sample.metadata and isinstance(sample.metadata.get("keywords"), list):
            keywords = [str(k) for k in sample.metadata.get("keywords") or []]
            if keywords:
                faithfulness = _compute_faithfulness(pred, keywords)
                faithfulness_total += faithfulness
                faithfulness_count += 1

        # Teacher agreement (if teacher model is available)
        if teacher_model is not None:
            teacher_out = _generate(
                tokenizer,
                teacher_model,
                sample.prompt,
                cfg.max_new_tokens,
                cfg.temperature,
                cfg.top_p,
            )
            teacher_norm = _normalize_text(teacher_out)
            if teacher_norm == pred_norm:
                teacher_agree += 1

        predictions.append(
            {
                "prompt": sample.prompt,
                "expected": sample.expected,
                "predicted": pred,
                "exact_match": exact,
                "fuzzy_match": fuzzy,
                "bleu": bleu,
                "f1": f1,
                "citation_coverage": coverage,
                "faithfulness": faithfulness,
            }
        )

        _jsonl(
            "progress",
            {
                "eval_id": cfg.eval_id,
                "processed": idx,
                "total": total,
                "exact_match": exact_matches / idx,
                "fuzzy_match": fuzzy_total / idx,
                "bleu": bleu_total / idx,
                "f1": f1_total / idx,
            },
        )

    # Compute final metrics
    exact_rate = exact_matches / total
    fuzzy_avg = fuzzy_total / total
    bleu_avg = bleu_total / total
    f1_avg = f1_total / total

    metrics = {
        "exact_match": exact_rate,
        "fuzzy_match": fuzzy_avg,
        "bleu": bleu_avg,
        "f1": f1_avg,
    }

    if coverage_count > 0:
        metrics["citation_coverage"] = coverage_total / coverage_count

    if faithfulness_count > 0:
        metrics["faithfulness"] = faithfulness_total / faithfulness_count

    if teacher_model is not None:
        metrics["teacher_agreement"] = teacher_agree / total

    predictions_path = run_dir / "predictions.jsonl"
    _atomic_write_jsonl(predictions_path, predictions)

    metrics_path = run_dir / "metrics.json"
    _atomic_write_text(metrics_path, json.dumps(metrics, indent=2, ensure_ascii=True))

    _jsonl("artifact", {"kind": "predictions", "path": str(predictions_path)})
    _jsonl("artifact", {"kind": "metrics", "path": str(metrics_path)})

    for name, value in metrics.items():
        _jsonl("metric", {"name": name, "value": value})


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", help="Path to JSON config")
    parser.add_argument("--stdin", action="store_true", help="Read JSON config from stdin")
    parser.add_argument("--run-dir", help="Override run_dir in config")
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
            {"level": "info", "message": "evaluator started", "eval_id": cfg.eval_id},
        )
        _jsonl(
            "env",
            {
                "python": sys.version.split()[0],
                "imports": _try_imports(),
                "gpu": _gpu_info(),
            },
        )

        db_path = Path(cfg.training_db_path) if cfg.training_db_path else _infer_training_db_path(run_dir)
        if not db_path:
            raise RuntimeError("training_db_path not provided and could not infer")

        samples = _load_dataset_items(db_path, cfg.dataset_id)
        _jsonl(
            "dataset",
            {"eval_id": cfg.eval_id, "total": len(samples), "dataset_id": cfg.dataset_id},
        )

        _evaluate(cfg, raw, run_dir, samples)

        _jsonl(
            "status",
            {"level": "info", "message": "evaluator completed", "eval_id": cfg.eval_id},
        )
        return 0
    except Exception as e:
        _jsonl(
            "status",
            {"level": "error", "message": str(e), "trace": _safe_exc()},
        )
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
