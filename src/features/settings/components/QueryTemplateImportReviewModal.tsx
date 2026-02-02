import { useMemo, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { open } from "@tauri-apps/plugin-dialog";
import {
  AlertCircle,
  Check,
  FileDown,
  FileText,
  Loader2,
  X,
} from "lucide-react";
import { cn } from "../../../utils/cn";
import type {
  QueryTemplateImportPreview,
  QueryTemplateImportPreviewItem,
  QueryTemplateInput,
} from "../../rag/types";
import {
  dbImportQueryTemplatesFromPreview,
  dbPreviewQueryTemplatesImportFromSqlFile,
} from "../../rag/api";

type Props = {
  isOpen: boolean;
  onClose: () => void;
  profileId: number;
  onImported: () => Promise<void>;
  addToast?: (message: string, type?: "success" | "error" | "info") => void;
};

function isErrorItem(item: QueryTemplateImportPreviewItem) {
  return item.issues.some((i) => i.startsWith("ERROR:"));
}

function isExactDuplicate(item: QueryTemplateImportPreviewItem) {
  return item.duplicate?.kind === "exact";
}

function isAnyDuplicate(item: QueryTemplateImportPreviewItem) {
  return Boolean(item.duplicate);
}

export function QueryTemplateImportReviewModal({
  isOpen,
  onClose,
  profileId,
  onImported,
  addToast,
}: Props) {
  const [filePath, setFilePath] = useState<string>("");
  const [preview, setPreview] = useState<QueryTemplateImportPreview | null>(null);
  const [expandedKey, setExpandedKey] = useState<string | null>(null);
  const [selectedKeys, setSelectedKeys] = useState<Set<string>>(new Set());

  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [onlyShowProblems, setOnlyShowProblems] = useState(false);
  const [showStatementErrors, setShowStatementErrors] = useState(false);

  const filteredItems = useMemo(() => {
    if (!preview) return [];
    const items = preview.items;
    if (!onlyShowProblems) return items;
    return items.filter((it) => isErrorItem(it) || it.duplicate);
  }, [preview, onlyShowProblems]);

  const defaultSelection = useMemo(() => {
    if (!preview) return new Set<string>();
    const next = new Set<string>();
    for (const it of preview.items) {
      if (!isErrorItem(it) && !isAnyDuplicate(it)) {
        next.add(it.key);
      }
    }
    return next;
  }, [preview]);

  const selectedCount = selectedKeys.size;

  const pickFileAndPreview = async () => {
    setError(null);
    setBusy(true);
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        title: "Select SQL templates file",
        filters: [{ name: "SQL", extensions: ["sql"] }],
      });

      const path =
        typeof selected === "string"
          ? selected
          : Array.isArray(selected)
            ? selected[0]
            : null;

      if (!path) {
        setBusy(false);
        return;
      }

      setFilePath(path);
      const pv = await dbPreviewQueryTemplatesImportFromSqlFile(path, profileId);
      setPreview(pv);
      {
        const next = new Set<string>();
        for (const it of pv.items) {
          if (!isErrorItem(it) && !isAnyDuplicate(it)) {
            next.add(it.key);
          }
        }
        setSelectedKeys(next);
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      addToast?.(`Preview failed: ${msg}`, "error");
    } finally {
      setBusy(false);
    }
  };

  const toggleKey = (key: string) => {
    setSelectedKeys((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const setAll = (on: boolean) => {
    if (!preview) return;
    if (!on) {
      setSelectedKeys(new Set());
      return;
    }
    const next = new Set<string>();
    for (const it of preview.items) {
      if (!isErrorItem(it) && !isAnyDuplicate(it)) next.add(it.key);
    }
    setSelectedKeys(next);
  };

  const importSelected = async () => {
    if (!preview) return;
    setError(null);
    setBusy(true);
    try {
      const items: QueryTemplateInput[] = preview.items
        .filter((it) => selectedKeys.has(it.key))
        .map((it) => it.template);

      const res = await dbImportQueryTemplatesFromPreview(profileId, items);
      addToast?.(
        `Imported ${res.imported}/${res.requested} (skipped duplicates: ${res.skipped_duplicates})`,
        "success",
      );
      await onImported();
      onClose();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      addToast?.(`Import failed: ${msg}`, "error");
    } finally {
      setBusy(false);
    }
  };

  if (!isOpen) return null;

  return (
    <AnimatePresence>
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
        onClick={(e) => e.target === e.currentTarget && onClose()}>
        <motion.div
          initial={{ opacity: 0, scale: 0.96, y: 16 }}
          animate={{ opacity: 1, scale: 1, y: 0 }}
          exit={{ opacity: 0, scale: 0.96, y: 16 }}
          className="bg-app-panel border border-app-border rounded-2xl shadow-2xl w-full max-w-4xl max-h-[90vh] overflow-hidden flex flex-col">
          <div className="px-6 py-4 border-b border-app-border flex items-center justify-between bg-app-card/30">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl bg-app-accent/10 flex items-center justify-center">
                <FileDown className="w-5 h-5 text-app-accent" />
              </div>
              <div>
                <h2 className="text-lg font-bold text-app-text">
                  Import Query Templates
                </h2>
                <p className="text-xs text-app-subtext">
                  Preview first to avoid duplicates / invalid templates
                </p>
              </div>
            </div>
            <button
              onClick={onClose}
              className="p-2 rounded-lg hover:bg-app-border/40 transition-colors text-app-subtext hover:text-app-text">
              <X className="w-5 h-5" />
            </button>
          </div>

          <div className="flex-1 overflow-y-auto p-6 space-y-4 custom-scrollbar">
            {error && (
              <div className="p-4 bg-destructive/10 border border-destructive/20 rounded-xl flex items-center gap-3 text-destructive">
                <AlertCircle className="w-5 h-5 shrink-0" />
                <p className="text-sm">{error}</p>
              </div>
            )}

            <div className="flex items-center justify-between gap-3">
              <div className="min-w-0">
                <div className="text-[10px] font-bold uppercase tracking-wider text-app-subtext mb-1">
                  SQL file
                </div>
                <div className="text-xs text-app-text truncate">
                  {filePath || "(not selected)"}
                </div>
              </div>
              <button
                type="button"
                onClick={pickFileAndPreview}
                disabled={busy}
                className={cn(
                  "px-4 py-2 rounded-xl text-sm font-bold flex items-center gap-2",
                  "bg-app-border/40 hover:bg-app-border transition-colors",
                  busy && "opacity-60 cursor-not-allowed",
                )}>
                {busy ? (
                  <Loader2 className="w-4 h-4 animate-spin" />
                ) : (
                  <FileText className="w-4 h-4" />
                )}
                Preview
              </button>
            </div>

            {preview && (
              <>
                <div className="p-4 bg-app-card/30 rounded-xl border border-app-border/60 flex flex-wrap items-center gap-3 text-xs">
                  <span className="text-app-subtext">
                    Parsed: <span className="text-app-text font-bold">{preview.parsed_count}</span>
                  </span>
                  <span className="text-app-subtext">
                    OK: <span className="text-app-success font-bold">{preview.ok_count}</span>
                  </span>
                  <span className="text-app-subtext">
                    Warnings: <span className="text-amber-400 font-bold">{preview.warning_count}</span>
                  </span>
                  <span className="text-app-subtext">
                    Errors: <span className="text-destructive font-bold">{preview.error_count}</span>
                  </span>
                  <span className="text-app-subtext">
                    Duplicates: <span className="text-app-subtext font-bold">{preview.duplicate_count}</span>
                  </span>
                  <span className="text-app-subtext">
                    Statement errors:{" "}
                    <span className="text-app-subtext font-bold">
                      {preview.statement_errors?.length || 0}
                    </span>
                  </span>
                  <span className="ml-auto flex items-center gap-2">
                    <label className="flex items-center gap-2 text-[11px] text-app-subtext cursor-pointer select-none">
                      <input
                        type="checkbox"
                        checked={onlyShowProblems}
                        onChange={(e) => setOnlyShowProblems(e.target.checked)}
                        className="w-4 h-4 rounded border-app-border bg-app-card"
                      />
                      Only show problems
                    </label>
                  </span>
                </div>

                {(preview.statement_errors?.length || 0) > 0 && (
                  <div className="p-4 bg-amber-500/5 border border-amber-500/20 rounded-xl">
                    <div className="flex items-center justify-between gap-3">
                      <div className="text-xs text-amber-300">
                        Some SQL statements could not be parsed/executed for preview.
                      </div>
                      <button
                        type="button"
                        onClick={() => setShowStatementErrors((v) => !v)}
                        className="px-3 py-1.5 rounded-lg text-xs font-bold bg-amber-500/10 hover:bg-amber-500/15 text-amber-300 transition-colors">
                        {showStatementErrors ? "Hide" : "Show"}
                      </button>
                    </div>
                    {showStatementErrors && (
                      <div className="mt-3 space-y-2">
                        {preview.statement_errors.map((msg, idx) => (
                          <div key={idx} className="text-[11px] font-mono text-amber-200/90">
                            {msg}
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                )}

                <div className="flex items-center justify-between gap-3">
                  <div className="text-xs text-app-subtext">
                    Selected: <span className="text-app-text font-bold">{selectedCount}</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <button
                      type="button"
                      onClick={() => setAll(true)}
                      disabled={busy}
                      className="px-3 py-1.5 rounded-lg text-xs font-bold bg-app-border/30 hover:bg-app-border/50 transition-colors">
                      Select all (safe)
                    </button>
                    <button
                      type="button"
                      onClick={() => setSelectedKeys(new Set(defaultSelection))}
                      disabled={busy}
                      className="px-3 py-1.5 rounded-lg text-xs font-bold bg-app-border/30 hover:bg-app-border/50 transition-colors">
                      Reset default
                    </button>
                    <button
                      type="button"
                      onClick={() => setAll(false)}
                      disabled={busy}
                      className="px-3 py-1.5 rounded-lg text-xs font-bold bg-app-border/30 hover:bg-app-border/50 transition-colors">
                      Clear
                    </button>
                  </div>
                </div>

                <div className="space-y-2">
                  {filteredItems.map((it) => {
                    const hasError = isErrorItem(it);
                    const isDup = Boolean(it.duplicate);
                    const isExactDup = isExactDuplicate(it);
                    const checked = selectedKeys.has(it.key);

                    return (
                      <div
                        key={it.key}
                        className={cn(
                          "border rounded-xl overflow-hidden",
                          hasError
                            ? "border-destructive/30 bg-destructive/5"
                            : isDup
                              ? "border-amber-500/30 bg-amber-500/5"
                              : "border-app-border bg-app-card/30",
                        )}>
                        <div className="px-4 py-3 flex items-start gap-3">
                          <input
                            type="checkbox"
                            checked={checked}
                            onChange={() => toggleKey(it.key)}
                            disabled={hasError || isExactDup || busy}
                            className="mt-1 w-4 h-4 rounded border-app-border bg-app-card"
                          />

                          <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-2">
                              <div className="text-sm font-bold text-app-text truncate">
                                {it.template.name}
                              </div>
                              {it.duplicate && (
                                <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-amber-500/20 text-amber-400">
                                  DUP:{it.duplicate.kind.toUpperCase()}
                                </span>
                              )}
                              {hasError && (
                                <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-destructive/20 text-destructive">
                                  ERROR
                                </span>
                              )}
                            </div>
                            <div className="mt-1 text-[11px] text-app-subtext">
                              type: <span className="font-mono">{it.template.pattern_type}</span>
                              {" "}• priority: {it.template.priority ?? 0}
                              {" "}• enabled: {String(it.template.is_enabled ?? true)}
                            </div>
                          </div>

                          <button
                            type="button"
                            onClick={() =>
                              setExpandedKey(expandedKey === it.key ? null : it.key)
                            }
                            className="px-3 py-1.5 rounded-lg text-xs font-bold bg-app-border/30 hover:bg-app-border/50 transition-colors">
                            {expandedKey === it.key ? "Hide" : "Review"}
                          </button>
                        </div>

                        <AnimatePresence>
                          {expandedKey === it.key && (
                            <motion.div
                              initial={{ height: 0, opacity: 0 }}
                              animate={{ height: "auto", opacity: 1 }}
                              exit={{ height: 0, opacity: 0 }}
                              className="border-t border-app-border">
                              <div className="p-4 space-y-3">
                                {it.duplicate && (
                                  <div className="text-xs text-amber-300">
                                    Conflicts with existing template #{it.duplicate.existing_template_id}:{" "}
                                    <span className="font-semibold">
                                      {it.duplicate.existing_template_name}
                                    </span>
                                  </div>
                                )}

                                {it.issues.length > 0 && (
                                  <div className="space-y-1">
                                    {it.issues.map((msg, idx) => (
                                      <div
                                        key={idx}
                                        className={cn(
                                          "text-xs font-mono",
                                          msg.startsWith("ERROR:")
                                            ? "text-destructive"
                                            : msg.startsWith("WARN:")
                                              ? "text-amber-400"
                                              : "text-app-subtext",
                                        )}>
                                        {msg}
                                      </div>
                                    ))}
                                  </div>
                                )}

                                <div>
                                  <div className="text-[10px] font-bold uppercase tracking-wider text-app-subtext mb-1">
                                    Query pattern
                                  </div>
                                  <pre className="p-3 bg-app-bg/30 border border-app-border/60 rounded-lg text-xs font-mono text-app-text overflow-x-auto whitespace-pre-wrap">
                                    {it.template.query_pattern}
                                  </pre>
                                </div>
                              </div>
                            </motion.div>
                          )}
                        </AnimatePresence>
                      </div>
                    );
                  })}

                  {filteredItems.length === 0 && (
                    <div className="p-6 text-center text-sm text-app-subtext">
                      No items to show.
                    </div>
                  )}
                </div>
              </>
            )}
          </div>

          <div className="px-6 py-4 border-t border-app-border bg-app-card/30 flex justify-between items-center gap-3">
            <div className="text-xs text-app-subtext">
              Target profile: <span className="font-bold text-app-text">{profileId}</span>
            </div>
            <div className="flex items-center gap-3">
              <button
                type="button"
                onClick={onClose}
                className="px-5 py-2.5 text-sm font-semibold text-app-subtext hover:text-app-text transition-colors">
                Cancel
              </button>
              <button
                type="button"
                onClick={importSelected}
                disabled={busy || !preview || selectedKeys.size === 0}
                className={cn(
                  "px-6 py-2.5 rounded-xl text-sm font-bold shadow-lg transition-all flex items-center gap-2",
                  "bg-app-accent text-white hover:bg-app-accent/90 hover:scale-[1.02] active:scale-[0.98]",
                  (busy || !preview || selectedKeys.size === 0) &&
                    "opacity-50 cursor-not-allowed",
                )}>
                {busy ? (
                  <Loader2 className="w-4 h-4 animate-spin" />
                ) : (
                  <Check className="w-4 h-4" />
                )}
                Import selected
              </button>
            </div>
          </div>
        </motion.div>
      </motion.div>
    </AnimatePresence>
  );
}
