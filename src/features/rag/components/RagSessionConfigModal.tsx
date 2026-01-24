import { useCallback, useMemo, useState } from "react";
import { ChevronDown, X } from "lucide-react";
import { PROVIDER_MODEL_OPTIONS } from "../../../store/settings";
import { cn } from "../../../utils/cn";

export function RagSessionConfigModal(props: {
  open: boolean;
  onClose: () => void;
  provider: string;
  model: string;
  setModel: (model: string) => void;
  localModels: string[];
  openRouterModels: string[] | undefined;
  answerLanguage: "id" | "en";
  setAnswerLanguage: (lang: "id" | "en") => void;
  strictRagMode: boolean;
  setStrictRagMode: (v: boolean) => void;
  topK: number;
  setTopK: (v: number) => void;
  candidateK: number;
  setCandidateK: (v: number) => void;
  rerankK: number;
  setRerankK: (v: number) => void;
}) {
  const {
    open,
    onClose,
    provider,
    model,
    setModel,
    localModels,
    openRouterModels,
    answerLanguage,
    setAnswerLanguage,
    strictRagMode,
    setStrictRagMode,
    topK,
    setTopK,
    candidateK,
    setCandidateK,
    rerankK,
    setRerankK,
  } = props;

  const [helpOpen, setHelpOpen] = useState<"top_k" | "candidate_k" | "rerank_k" | null>(
    null,
  );
  const [helpLang, setHelpLang] = useState<"id" | "en">("id");
  const [sessionPreset, setSessionPreset] = useState<
    "balanced" | "local_4k" | "csv_xlsx" | "pdf" | "txt_md" | "custom"
  >("balanced");
  const [showAdvanced, setShowAdvanced] = useState(false);

  const applyPreset = useCallback(
    (preset: "local_4k" | "pdf" | "csv_xlsx" | "txt_md" | "balanced") => {
      if (preset === "local_4k") {
        setTopK(3);
        setCandidateK(60);
        setRerankK(40);
        return;
      }
      if (preset === "csv_xlsx") {
        setTopK(10);
        setCandidateK(60);
        setRerankK(40);
        return;
      }
      if (preset === "pdf") {
        setTopK(6);
        setCandidateK(120);
        setRerankK(80);
        return;
      }
      if (preset === "txt_md") {
        setTopK(6);
        setCandidateK(100);
        setRerankK(75);
        return;
      }
      setTopK(5);
      setCandidateK(100);
      setRerankK(75);
    },
    [setTopK, setCandidateK, setRerankK],
  );

  const helpTitle =
    helpOpen === "top_k"
      ? "top_k (context size)"
      : helpOpen === "candidate_k"
        ? "candidate_k (recall pool)"
        : helpOpen === "rerank_k"
          ? "rerank_k (reranker input)"
          : "";

  const helpBody = useMemo(() => {
    const en =
      helpOpen === "top_k"
        ? "Controls how many context chunks are sent to the LLM. Higher can improve answer quality, but increases prompt size and can hit local model context limits (e.g. 4k). For 4k-context local models, use 1-3."
        : helpOpen === "candidate_k"
          ? "Controls how many chunks are recalled before selecting final context. Higher improves recall (more chances to find the right chunk) but costs more CPU and can slow retrieval. Typical: 50-200. Must be >= top_k."
          : helpOpen === "rerank_k"
            ? "Controls how many recalled candidates are reranked by the local reranker model (fastembed TextRerank). Higher can improve ordering/precision, but costs more time. Typical: 25-100. Must be between top_k and candidate_k."
            : "";
    const id =
      helpOpen === "top_k"
        ? "Mengatur jumlah potongan konteks (chunk) yang dikirim ke LLM. Semakin tinggi biasanya jawabannya lebih akurat, tapi prompt jadi lebih panjang dan bisa kena limit konteks model lokal (mis. 4k). Untuk model lokal 4k, pakai 1-3."
        : helpOpen === "candidate_k"
          ? "Mengatur berapa banyak kandidat chunk yang di-recall sebelum dipilih jadi konteks final. Semakin tinggi recall makin bagus (peluang ketemu chunk yang tepat naik), tapi CPU lebih berat dan retrieval bisa lebih lama. Umum: 50-200. Harus >= top_k."
          : helpOpen === "rerank_k"
            ? "Mengatur berapa banyak kandidat yang di-rerank oleh reranker lokal (fastembed TextRerank). Semakin tinggi biasanya urutan hasil lebih tepat, tapi proses lebih lama. Umum: 25-100. Harus di antara top_k dan candidate_k."
            : "";
    return helpLang === "en" ? en : id;
  }, [helpLang, helpOpen]);

  if (!open) return null;

  const openRouterEffectiveModels =
    (openRouterModels && openRouterModels.length > 0
      ? openRouterModels
      : PROVIDER_MODEL_OPTIONS.openrouter) ?? [];

  return (
    <>
      <div className="fixed inset-0 z-50 flex items-center justify-center px-4">
        <button className="absolute inset-0 bg-black/50" onClick={onClose} aria-label="Close session config" />
        <div className="relative w-full max-w-2xl rounded-2xl bg-app-card border border-app-border/60 shadow-2xl p-5">
          <div className="flex items-start justify-between gap-3">
            <div>
              <div className="text-base font-semibold text-app-text">Session settings</div>
              <div className="text-xs text-app-text-muted mt-1 leading-relaxed">
                Model, chat behavior, and retrieval knobs for this conversation.
              </div>
            </div>
            <button
              onClick={onClose}
              className="p-1.5 rounded-lg hover:bg-app-bg/40 text-app-text-muted hover:text-app-text transition-colors"
              aria-label="Close">
              <X className="w-4 h-4" />
            </button>
          </div>

          <div className="mt-4 grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="rounded-xl bg-app-bg/20 p-4">
              <div className="text-sm font-medium text-app-text">Presets</div>
              <div className="text-xs text-app-text-muted mt-1 leading-relaxed">
                Choose a sensible default for your data and model.
              </div>
              <div className="mt-3 grid grid-cols-1 gap-2">
                <button
                  type="button"
                  onClick={() => {
                    setSessionPreset("local_4k");
                    applyPreset("local_4k");
                  }}
                  className={cn(
                    "w-full text-left px-3 py-2 rounded-lg transition-colors",
                    sessionPreset === "local_4k"
                      ? "bg-app-accent/10 ring-1 ring-app-accent/25"
                      : "hover:bg-app-bg/30",
                  )}>
                  <div className="text-sm font-medium text-app-text">Local / 4k context</div>
                  <div className="text-xs text-app-text-muted mt-0.5">Safer for short context windows.</div>
                </button>

                <button
                  type="button"
                  onClick={() => {
                    setSessionPreset("csv_xlsx");
                    applyPreset("csv_xlsx");
                  }}
                  className={cn(
                    "w-full text-left px-3 py-2 rounded-lg transition-colors",
                    sessionPreset === "csv_xlsx"
                      ? "bg-app-accent/10 ring-1 ring-app-accent/25"
                      : "hover:bg-app-bg/30",
                  )}>
                  <div className="text-sm font-medium text-app-text">CSV / XLSX</div>
                  <div className="text-xs text-app-text-muted mt-0.5">Best for list/aggregate queries.</div>
                </button>

                <button
                  type="button"
                  onClick={() => {
                    setSessionPreset("pdf");
                    applyPreset("pdf");
                  }}
                  className={cn(
                    "w-full text-left px-3 py-2 rounded-lg transition-colors",
                    sessionPreset === "pdf"
                      ? "bg-app-accent/10 ring-1 ring-app-accent/25"
                      : "hover:bg-app-bg/30",
                  )}>
                  <div className="text-sm font-medium text-app-text">PDF / DOCX</div>
                  <div className="text-xs text-app-text-muted mt-0.5">Higher recall + rerank.</div>
                </button>

                <button
                  type="button"
                  onClick={() => {
                    setSessionPreset("txt_md");
                    applyPreset("txt_md");
                  }}
                  className={cn(
                    "w-full text-left px-3 py-2 rounded-lg transition-colors",
                    sessionPreset === "txt_md"
                      ? "bg-app-accent/10 ring-1 ring-app-accent/25"
                      : "hover:bg-app-bg/30",
                  )}>
                  <div className="text-sm font-medium text-app-text">TXT / Markdown</div>
                  <div className="text-xs text-app-text-muted mt-0.5">Keyword-friendly retrieval.</div>
                </button>

                <button
                  type="button"
                  onClick={() => {
                    setSessionPreset("balanced");
                    applyPreset("balanced");
                  }}
                  className={cn(
                    "w-full text-left px-3 py-2 rounded-lg transition-colors",
                    sessionPreset === "balanced"
                      ? "bg-app-accent/10 ring-1 ring-app-accent/25"
                      : "hover:bg-app-bg/30",
                  )}>
                  <div className="text-sm font-medium text-app-text">Balanced</div>
                  <div className="text-xs text-app-text-muted mt-0.5">Default behavior.</div>
                </button>

                {sessionPreset === "custom" && (
                  <div className="px-3 py-2 rounded-lg bg-app-bg/20">
                    <div className="text-xs font-medium text-app-text">Custom</div>
                    <div className="text-xs text-app-text-muted mt-1">
                      top_k={topK} | candidate_k={candidateK} | rerank_k={rerankK}
                    </div>
                  </div>
                )}
              </div>
            </div>

            <div className="rounded-xl bg-app-bg/20 p-4 space-y-5">
              <div>
                <div className="text-sm font-medium text-app-text mb-2">Model</div>
                <div className="space-y-1.5">
                  <label className="text-[10px] text-app-text-muted font-medium ml-1">
                    Model ({provider})
                  </label>
                  <div className="relative">
                    <select
                      value={model}
                      onChange={(e) => setModel(e.target.value)}
                      className="w-full appearance-none bg-app-card border border-app-border/60 rounded-lg py-2 pl-3 pr-8 text-xs outline-none focus:border-app-accent/50 focus:ring-1 focus:ring-app-accent/20 transition-all text-app-text font-medium">
                      {provider === "local" || provider === "ollama" || provider === "llama_cpp" ? (
                        localModels && localModels.length > 0 ? (
                          localModels.map((m) => (
                            <option key={m} value={m}>
                              {m}
                            </option>
                          ))
                        ) : (
                          <option value={model} disabled>
                            No local models found
                          </option>
                        )
                      ) : provider === "gemini" ? (
                        (PROVIDER_MODEL_OPTIONS.gemini ?? []).map((m) => (
                          <option key={m} value={m}>
                            {m}
                          </option>
                        ))
                      ) : provider === "openai" ? (
                        (PROVIDER_MODEL_OPTIONS.openai ?? []).map((m) => (
                          <option key={m} value={m}>
                            {m}
                          </option>
                        ))
                      ) : provider === "openrouter" ? (
                        openRouterEffectiveModels.map((m) => (
                          <option key={m} value={m}>
                            {m}
                          </option>
                        ))
                      ) : (
                        <option value={model}>{model || "Custom Model"}</option>
                      )}
                    </select>
                    <ChevronDown className="absolute right-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-app-text-muted pointer-events-none" />
                  </div>
                </div>
              </div>

              <div>
                <div className="text-sm font-medium text-app-text mb-2">Chat</div>
                <div className="space-y-3">
                  <div className="space-y-1.5">
                    <label className="text-[10px] text-app-text-muted font-medium ml-1">
                      Response Language
                    </label>
                    <div className="flex items-center gap-1 p-1 bg-app-card border border-app-border/60 rounded-lg">
                      <button
                        onClick={() => setAnswerLanguage("en")}
                        className={cn(
                          "flex-1 text-center py-1.5 text-[10px] font-semibold rounded-md transition-all flex items-center justify-center gap-1.5",
                          answerLanguage === "en"
                            ? "bg-app-accent text-white shadow-sm"
                            : "text-app-text-muted hover:text-app-text hover:bg-app-bg/50",
                        )}>
                        English
                      </button>
                      <button
                        onClick={() => setAnswerLanguage("id")}
                        className={cn(
                          "flex-1 text-center py-1.5 text-[10px] font-semibold rounded-md transition-all flex items-center justify-center gap-1.5",
                          answerLanguage === "id"
                            ? "bg-app-accent text-white shadow-sm"
                            : "text-app-text-muted hover:text-app-text hover:bg-app-bg/50",
                        )}>
                        Indonesia
                      </button>
                    </div>
                  </div>

                  <div className="space-y-1.5">
                    <label className="text-[10px] text-app-text-muted font-medium ml-1">
                      Chat Mode
                    </label>
                    <div className="flex items-center gap-1 p-1 bg-app-card border border-app-border/60 rounded-lg">
                      <button
                        onClick={() => setStrictRagMode(false)}
                        className={cn(
                          "flex-1 text-center py-1.5 text-[10px] font-semibold rounded-md transition-all",
                          !strictRagMode
                            ? "bg-app-accent text-white shadow-sm"
                            : "text-app-text-muted hover:text-app-text hover:bg-app-bg/50",
                        )}>
                        Chatbot
                      </button>
                      <button
                        onClick={() => setStrictRagMode(true)}
                        className={cn(
                          "flex-1 text-center py-1.5 text-[10px] font-semibold rounded-md transition-all",
                          strictRagMode
                            ? "bg-app-accent text-white shadow-sm"
                            : "text-app-text-muted hover:text-app-text hover:bg-app-bg/50",
                        )}>
                        Strict RAG
                      </button>
                    </div>
                    <div className="text-[10px] text-app-text-muted/70 px-1 leading-relaxed">
                      {strictRagMode
                        ? "Strict RAG: only answer from local context (except greetings)."
                        : "Chatbot: can answer generally when context is missing."}
                    </div>
                  </div>
                </div>
              </div>

              <div>
                <div className="flex items-center justify-between gap-2 mb-2">
                  <div className="text-sm font-medium text-app-text">Retrieval</div>
                  <div className="flex items-center gap-3">
                    <button
                      type="button"
                      onClick={() => {
                        setHelpLang(answerLanguage);
                        setHelpOpen("top_k");
                      }}
                      className="text-xs text-app-text-muted hover:text-app-text transition-colors">
                      Help
                    </button>
                    <button
                      type="button"
                      onClick={() => setShowAdvanced((v) => !v)}
                      className="text-xs font-medium text-app-text hover:opacity-90 transition-opacity">
                      {showAdvanced ? "Hide" : "Advanced"}
                    </button>
                  </div>
                </div>

                <div className="text-xs text-app-text-muted mb-3 px-1">
                  Current: top_k={topK} | candidate_k={candidateK} | rerank_k={rerankK}
                </div>

                {showAdvanced && (
                  <div className="space-y-3">
                    <div className="space-y-1.5">
                      <label className="text-[10px] text-app-text-muted font-medium ml-1">
                        Context Size
                      </label>
                      <div className="space-y-1">
                        <div className="text-[10px] text-app-text-muted/70 px-1">top_k</div>
                        <input
                          type="number"
                          min={1}
                          value={topK}
                          onChange={(e) => {
                            setSessionPreset("custom");
                            const v = parseInt(e.target.value, 10);
                            if (Number.isNaN(v)) return;
                            const next = Math.max(1, v);
                            setTopK(next);
                            if (candidateK < next) setCandidateK(next);
                            if (rerankK < next) setRerankK(next);
                          }}
                          className="w-full bg-app-card border border-app-border/60 rounded-lg py-2 px-3 text-xs outline-none focus:border-app-accent/50 focus:ring-1 focus:ring-app-accent/20 transition-all text-app-text font-medium"
                        />
                      </div>
                      <div className="text-[10px] text-app-text-muted/70 px-1 leading-relaxed">
                        For 4k-context local models, use 1-3.
                      </div>
                    </div>

                    <div className="space-y-1.5">
                      <label className="text-[10px] text-app-text-muted font-medium ml-1">
                        Retrieval Tuning
                      </label>
                      <div className="grid grid-cols-2 gap-2">
                        <div className="space-y-1">
                          <div className="text-[10px] text-app-text-muted/70 px-1">candidate_k</div>
                          <input
                            type="number"
                            min={1}
                            value={candidateK}
                            onChange={(e) => {
                              setSessionPreset("custom");
                              const v = parseInt(e.target.value, 10);
                              if (Number.isNaN(v)) return;
                              const next = Math.max(1, v);
                              if (next < topK) {
                                setCandidateK(topK);
                                if (rerankK > topK) setRerankK(topK);
                                return;
                              }
                              setCandidateK(next);
                              if (rerankK > next) setRerankK(next);
                            }}
                            className="w-full bg-app-card border border-app-border/60 rounded-lg py-2 px-3 text-xs outline-none focus:border-app-accent/50 focus:ring-1 focus:ring-app-accent/20 transition-all text-app-text font-medium"
                          />
                        </div>
                        <div className="space-y-1">
                          <div className="text-[10px] text-app-text-muted/70 px-1">rerank_k</div>
                          <input
                            type="number"
                            min={topK}
                            max={candidateK}
                            value={rerankK}
                            onChange={(e) => {
                              setSessionPreset("custom");
                              const v = parseInt(e.target.value, 10);
                              if (Number.isNaN(v)) return;
                              setRerankK(Math.max(topK, Math.min(candidateK, v)));
                            }}
                            className="w-full bg-app-card border border-app-border/60 rounded-lg py-2 px-3 text-xs outline-none focus:border-app-accent/50 focus:ring-1 focus:ring-app-accent/20 transition-all text-app-text font-medium"
                          />
                        </div>
                      </div>
                      <div className="text-[10px] text-app-text-muted/70 px-1 leading-relaxed">
                        Higher values can improve recall, but cost more CPU.
                      </div>
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>

          <div className="mt-4 flex items-center justify-end gap-2">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 rounded-lg bg-app-bg/30 border border-app-border/50 text-xs font-semibold text-app-text-muted hover:text-app-text hover:bg-app-bg/50 transition-all">
              Done
            </button>
          </div>
        </div>
      </div>

      {helpOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center px-4">
          <button className="absolute inset-0 bg-black/50" onClick={() => setHelpOpen(null)} aria-label="Close help" />
          <div className="relative w-full max-w-md rounded-2xl bg-app-card border border-app-border/60 shadow-2xl p-5">
            <div className="flex items-start justify-between gap-3">
              <div>
                <div className="text-xs font-bold uppercase tracking-wider text-app-text-muted">
                  Retrieval Help
                </div>
                <div className="text-base font-semibold text-app-text mt-1">{helpTitle}</div>
              </div>
              <div className="flex items-center gap-2">
                <div className="flex items-center gap-1 p-1 bg-app-bg/30 border border-app-border/40 rounded-lg">
                  <button
                    type="button"
                    onClick={() => setHelpLang("id")}
                    className={cn(
                      "px-2 py-1 text-[10px] font-bold rounded-md transition-all",
                      helpLang === "id"
                        ? "bg-app-accent text-white"
                        : "text-app-text-muted hover:text-app-text hover:bg-app-bg/40",
                    )}>
                    ID
                  </button>
                  <button
                    type="button"
                    onClick={() => setHelpLang("en")}
                    className={cn(
                      "px-2 py-1 text-[10px] font-bold rounded-md transition-all",
                      helpLang === "en"
                        ? "bg-app-accent text-white"
                        : "text-app-text-muted hover:text-app-text hover:bg-app-bg/40",
                    )}>
                    EN
                  </button>
                </div>
                <button
                  onClick={() => setHelpOpen(null)}
                  className="p-1.5 rounded-lg hover:bg-app-bg/40 text-app-text-muted hover:text-app-text transition-colors"
                  aria-label="Close">
                  <X className="w-4 h-4" />
                </button>
              </div>
            </div>
            <div className="mt-3 text-sm leading-6 text-app-text-muted">{helpBody}</div>
          </div>
        </div>
      )}
    </>
  );
}
