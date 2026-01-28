import { useState } from "react";
import {
  Copy,
  RotateCcw,
  ChevronDown,
  ChevronUp,
  Check,
  RectangleEllipsis,
  X,
  Maximize2,
  Code,
} from "lucide-react";
import { cn } from "../../../utils/cn";
import { MarkdownRenderer } from "./MarkdownRenderer";
import type { ChatMessage, RagQueryResult, TemplateMatch } from "../types";
// import { isLowConfidenceSources } from "../ragChatUtils";

interface MessageItemProps {
  message: ChatMessage;
  onRegenerate: (query: string) => void;
  /** Regenerate with a specific template (for DB collections) */
  onRegenerateWithTemplate?: (
    query: string,
    templateId: number,
    autoSelectedTemplateId?: number
  ) => void;
  onCopy: (text: string, id: string) => void;
  copiedId: string | null;
}

export function MessageItem({
  message,
  onRegenerate,
  onRegenerateWithTemplate,
  onCopy,
  copiedId,
}: MessageItemProps) {
  const [showSources, setShowSources] = useState(false);
  const [showTemplates, setShowTemplates] = useState(false);
  const [showSqlQuery, setShowSqlQuery] = useState(false);
  const [openSourceIndex, setOpenSourceIndex] = useState<number | null>(null);
  const isUser = message.type === "user";

  // Check if this is a DB message with templates
  const hasTemplates =
    message.telemetry?.matched_templates &&
    message.telemetry.matched_templates.length > 0;
  const selectedTemplateId = message.telemetry?.template_id;

  return (
    <div
      className={cn(
        "group w-full max-w-5xl mx-auto flex flex-col gap-2",
        isUser ? "items-end" : "items-start",
      )}>
      {/* Header Info */}
      <div
        className={cn(
          "flex items-center gap-2 px-1 opacity-60",
          isUser ? "flex-row-reverse" : "flex-row",
        )}>
        <span className="text-[10px] font-bold uppercase tracking-widest text-app-subtext">
          {isUser ? "You" : "Assistant"}
        </span>
        <span className="text-[10px] text-app-border">•</span>
        <span className="text-[10px] text-app-subtext">
          {new Date(message.timestamp).toLocaleTimeString([], {
            hour: "2-digit",
            minute: "2-digit",
          })}
        </span>
      </div>

      {/* Message Content */}
      <div
        className={cn(
          "relative max-w-[92%] transition-all duration-300",
          isUser
            ? "bg-primary/20 text-foreground border border-primary/30 rounded-2xl rounded-tr-sm p-4 shadow-sm"
            : "w-full",
        )}>
        {isUser ? (
          <p className="text-[14px] leading-relaxed whitespace-pre-wrap select-text font-medium">
            {message.content}
          </p>
        ) : (
          <div className="space-y-4">
            <div className="text-[15px] leading-relaxed text-app-text select-text prose prose-invert max-w-none prose-p:leading-relaxed prose-pre:bg-app-card prose-pre:border prose-pre:border-app-border/40 prose-code:text-app-accent">
              <MarkdownRenderer content={message.content} />
            </div>

            {/* Telemetry/Metatdata */}
            {message.telemetry && (
              <div className="flex flex-wrap items-center gap-3 pt-4 border-t border-app-border/10">
                <div className="flex items-center gap-2 px-2 py-1 rounded-md bg-app-card border border-app-border/40">
                  <span className="text-[10px] font-bold text-app-subtext uppercase">
                    Rows
                  </span>
                  <span className="text-[11px] font-medium text-app-text">
                    {message.telemetry.row_count}
                  </span>
                </div>
                {message.telemetry.latency_ms && (
                  <div className="flex items-center gap-2 px-2 py-1 rounded-md bg-app-card border border-app-border/40">
                    <span className="text-[10px] font-bold text-app-subtext uppercase">
                      Latency
                    </span>
                    <span className="text-[11px] font-medium text-app-text">
                      {message.telemetry.latency_ms} ms
                    </span>
                  </div>
                )}

                {/* SQL Query Toggle Button */}
                {(message.telemetry.executedSql || message.telemetry.query_plan) && (
                  <button
                    onClick={() => setShowSqlQuery(!showSqlQuery)}
                    className="flex items-center gap-2 text-[10px] font-bold text-amber-500/80 hover:text-amber-400 bg-amber-500/5 px-2 py-1 rounded-md transition-all border border-amber-500/10">
                    <Code className="w-3.5 h-3.5" />
                    <span>{showSqlQuery ? "Hide SQL" : "View SQL"}</span>
                  </button>
                )}

                {/* Template Selection (Feature 31 Enhancement) */}
                {hasTemplates && (
                  <div className="w-full mt-2">
                    <button
                      onClick={() => setShowTemplates(!showTemplates)}
                      className="flex items-center gap-2 text-[10px] font-bold text-primary/80 hover:text-primary bg-primary/5 px-3 py-1.5 rounded-lg transition-all border border-primary/10 w-full justify-between">
                      <div className="flex items-center gap-2">
                        <RectangleEllipsis className="w-3.5 h-3.5" />
                        <span>Try different query pattern</span>
                        <span className="px-1.5 py-0.5 rounded-full bg-primary/10 text-primary font-bold">
                          {message.telemetry!.matched_templates!.length}
                        </span>
                      </div>
                      {showTemplates ? (
                        <ChevronUp className="w-3.5 h-3.5" />
                      ) : (
                        <ChevronDown className="w-3.5 h-3.5" />
                      )}
                    </button>

                    {showTemplates && (
                      <div className="mt-2 space-y-2 animate-in fade-in slide-in-from-top-2 duration-200">
                        {message.telemetry!.matched_templates!.map((tpl) => (
                          <TemplateOption
                            key={tpl.template_id}
                            template={tpl}
                            isSelected={tpl.template_id === selectedTemplateId}
                            onSelect={() => {
                              if (message.query && onRegenerateWithTemplate) {
                                onRegenerateWithTemplate(
                                  message.query,
                                  tpl.template_id,
                                  selectedTemplateId ?? undefined,
                                );
                              }
                            }}
                          />
                        ))}
                      </div>
                    )}
                  </div>
                )}

                {/* Collapsible SQL Query */}
                {showSqlQuery && (message.telemetry.executedSql || message.telemetry.query_plan) && (
                  <div className="w-full mt-2 animate-in fade-in slide-in-from-top-2 duration-200">
                    <div className="relative group/sql">
                      <div className="flex items-center justify-between mb-2">
                        <span className="text-[10px] font-bold text-amber-500 uppercase flex items-center gap-1.5">
                          <Code className="w-3 h-3" />
                          Executed SQL Query
                        </span>
                        <button
                          onClick={() => onCopy(
                            message.telemetry!.executedSql || message.telemetry!.query_plan!,
                            `sql-${message.id}`
                          )}
                          className="p-1.5 rounded-md hover:bg-app-bg/40 text-app-subtext hover:text-app-accent transition-all opacity-0 group-hover/sql:opacity-100">
                          {copiedId === `sql-${message.id}` ? (
                            <Check className="w-3.5 h-3.5 text-emerald-400" />
                          ) : (
                            <Copy className="w-3.5 h-3.5" />
                          )}
                        </button>
                      </div>
                      <div className="rounded-xl bg-app-card border border-amber-500/20 overflow-hidden">
                        <pre className="p-3 text-[11px] font-mono text-amber-100/90 whitespace-pre-wrap break-words bg-amber-950/30">
                          {message.telemetry.executedSql || message.telemetry.query_plan}
                        </pre>
                      </div>
                    </div>
                  </div>
                )}
              </div>
            )}

            {/* Sources Toggle */}
            {message.sources && message.sources.length > 0 && (
              <div className="space-y-3">
                <button
                  onClick={() => setShowSources(!showSources)}
                  className="flex items-center gap-2 text-[11px] font-bold text-primary/80 hover:text-primary bg-primary/5 px-3 py-1.5 rounded-full transition-all border border-primary/10 uppercase tracking-widest">
                  {showSources ? (
                    <ChevronUp className="w-3.5 h-3.5" />
                  ) : (
                    <ChevronDown className="w-3.5 h-3.5" />
                  )}
                  {showSources
                    ? "Hide Sources"
                    : `View ${message.sources.length} Sources`}
                  {/* {!showSources && isLowConfidenceSources(message.sources) && (
                    <span className="ml-1 text-[9px] text-amber-500 font-bold border-l border-amber-500/20 pl-2">LOW CONFIDENCE</span>
                  )} */}
                </button>

                {showSources && (
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-3 animate-in fade-in slide-in-from-top-2 duration-300">
                    {message.sources.map((source, idx) => (
                      <SourceCard
                        key={idx}
                        source={source}
                        onCopy={(txt) =>
                          onCopy(txt, `src-${message.id}-${idx}`)
                        }
                        isCopied={copiedId === `src-${message.id}-${idx}`}
                        onViewFull={() => setOpenSourceIndex(idx)}
                      />
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        {/* Action Buttons */}
        <div
          className={cn(
            "absolute top-0 flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-all duration-200",
            isUser ? "right-full mr-2" : "left-full ml-2",
          )}>
          <button
            onClick={() => onCopy(message.content, message.id)}
            className="p-2 rounded-xl hover:bg-app-card text-app-subtext hover:text-app-accent transition-all border border-transparent hover:border-app-border/40">
            {copiedId === message.id ? (
              <span className="text-[10px] font-bold text-emerald-400">
                Copied
              </span>
            ) : (
              <Copy className="w-4 h-4" />
            )}
          </button>
          {!isUser && message.query && (
            <button
              onClick={() => onRegenerate(message.query!)}
              className="p-2 rounded-xl hover:bg-app-card text-app-subtext hover:text-app-accent transition-all border border-transparent hover:border-app-border/40">
              <RotateCcw className="w-4 h-4" />
            </button>
          )}
        </div>
      </div>

      {/* Source Modal */}
      {openSourceIndex !== null && message.sources && (
        <SourceModal
          source={message.sources[openSourceIndex]}
          open={true}
          onClose={() => setOpenSourceIndex(null)}
          onCopy={(txt) => onCopy(txt, `src-modal-${message.id}-${openSourceIndex}`)}
          isCopied={copiedId === `src-modal-${message.id}-${openSourceIndex}`}
        />
      )}
    </div>
  );
}

function SourceCard({
  source,
  onCopy,
  isCopied,
  onViewFull,
}: {
  source: RagQueryResult;
  onCopy: (txt: string) => void;
  isCopied: boolean;
  onViewFull: () => void;
}) {
  return (
    <div className="bg-app-card border border-app-border/30 rounded-xl p-3 space-y-2 hover:border-primary/30 transition-all group/src">
      <div className="flex items-center justify-between gap-2">
        <div className="flex flex-col min-w-0">
          <span className="text-[9px] font-bold text-app-subtext uppercase tracking-widest truncate">
            {source.source_type} • {source.doc_name}
          </span>
          {source.score !== null && source.score !== undefined && (
            <div className="flex items-center gap-1 mt-0.5">
              <div className="h-1 w-12 bg-app-bg rounded-full overflow-hidden">
                <div
                  className={cn(
                    "h-full rounded-full transition-all",
                    source.score > 0.4
                      ? "bg-emerald-500"
                      : source.score > 0.2
                        ? "bg-amber-500"
                        : "bg-red-500",
                  )}
                  style={{ width: `${source.score * 100}%` }}
                />
              </div>
              <span className="text-[9px] font-bold text-app-subtext">
                {Math.round(source.score * 100)}% Match
              </span>
            </div>
          )}
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={onViewFull}
            className="p-1.5 opacity-0 group-hover/src:opacity-100 hover:text-app-accent transition-all"
            title="View full source">
            <Maximize2 className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={() => onCopy(source.content)}
            className="p-1.5 opacity-0 group-hover/src:opacity-100 hover:text-app-accent transition-all">
            {isCopied ? (
              <span className="text-[9px] text-emerald-500 font-bold">Done</span>
            ) : (
              <Copy className="w-3.5 h-3.5" />
            )}
          </button>
        </div>
      </div>
      <p className="text-[11px] text-app-subtext italic line-clamp-3 leading-relaxed whitespace-pre-wrap px-2 border-l-2 border-app-border/40">
        "{source.content}"
      </p>
    </div>
  );
}

/** Source Modal for viewing full source content */
function SourceModal(props: {
  source: RagQueryResult;
  open: boolean;
  onClose: () => void;
  onCopy: (txt: string) => void;
  isCopied: boolean;
}) {
  const { source, open, onClose, onCopy, isCopied } = props;

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center px-4">
      <button
        className="absolute inset-0 bg-black/50"
        onClick={onClose}
        aria-label="Close source modal"
      />
      <div className="relative w-full max-w-3xl rounded-2xl bg-app-card border border-app-border/60 shadow-2xl p-5 max-h-[80vh] flex flex-col">
        <div className="flex items-start justify-between gap-3">
          <div className="flex-1 min-w-0">
            <div className="text-sm font-semibold text-app-text">Source Details</div>
            <div className="text-xs text-app-subtext mt-1 leading-relaxed">
              {source.source_type}
              {source.doc_name && ` • ${source.doc_name}`}
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg hover:bg-app-bg/40 text-app-subtext hover:text-app-text transition-colors shrink-0"
            aria-label="Close"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Metadata */}
        <div className="mt-4 flex flex-wrap items-center gap-3">
          {source.score !== null && source.score !== undefined && (
            <div className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-app-bg/20">
              <span className="text-[10px] font-bold text-app-subtext uppercase">
                Match Score
              </span>
              <div className="flex items-center gap-1">
                <div className="h-1.5 w-16 bg-app-bg rounded-full overflow-hidden">
                  <div
                    className={cn(
                      "h-full rounded-full",
                      source.score > 0.4
                        ? "bg-emerald-500"
                        : source.score > 0.2
                          ? "bg-amber-500"
                          : "bg-red-500",
                    )}
                    style={{ width: `${source.score * 100}%` }}
                  />
                </div>
                <span className="text-[11px] font-bold text-app-text">
                  {Math.round(source.score * 100)}%
                </span>
              </div>
            </div>
          )}

          {source.page_number !== null && (
            <div className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-app-bg/20">
              <span className="text-[10px] font-bold text-app-subtext uppercase">
                Page
              </span>
              <span className="text-[11px] font-medium text-app-text">
                {source.page_number}
              </span>
            </div>
          )}

          <div className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-app-bg/20">
            <span className="text-[10px] font-bold text-app-subtext uppercase">
              Source ID
            </span>
            <span className="text-[11px] font-medium text-app-text">
              #{source.source_id}
            </span>
          </div>
        </div>

        {/* Content */}
        <div className="mt-4 flex-1 overflow-y-auto">
          <div className="rounded-xl bg-app-bg/30 p-4 border border-app-border/30">
            <p className="text-sm text-app-text leading-relaxed whitespace-pre-wrap">
              {source.content}
            </p>
          </div>
        </div>

        {/* Actions */}
        <div className="mt-4 flex items-center justify-end gap-2">
          <button
            type="button"
            onClick={() => onCopy(source.content)}
            className={cn(
              "px-4 py-2 rounded-lg text-xs font-semibold transition-all flex items-center gap-2",
              isCopied
                ? "bg-emerald-500/10 text-emerald-400 border border-emerald-500/20"
                : "bg-app-bg/30 border border-app-border/50 text-app-subtext hover:text-app-text hover:bg-app-bg/50",
            )}
          >
            {isCopied ? (
              <>
                <Check className="w-3.5 h-3.5" />
                Copied
              </>
            ) : (
              <>
                <Copy className="w-3.5 h-3.5" />
                Copy Content
              </>
            )}
          </button>
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-2 rounded-lg bg-app-bg/30 border border-app-border/50 text-xs font-semibold text-app-subtext hover:text-app-text hover:bg-app-bg/50 transition-all"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}

/** Template option for user selection */
function TemplateOption({
  template,
  isSelected,
  onSelect,
}: {
  template: TemplateMatch;
  isSelected: boolean;
  onSelect: () => void;
}) {
  return (
    <div
      className={cn(
        "p-3 rounded-lg border transition-all cursor-pointer",
        isSelected
          ? "bg-emerald-500/10 border-emerald-500/30"
          : "bg-app-card border border-app-border/20 hover:border-primary/30 hover:bg-primary/5",
      )}
      onClick={onSelect}>
      <div className="flex items-start justify-between gap-2">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            {isSelected ? (
              <Check className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
            ) : (
              <div className="w-3.5 h-3.5 rounded-full border border-app-border/40 shrink-0" />
            )}
            <span
              className={cn(
                "text-[11px] font-medium",
                isSelected ? "text-emerald-400" : "text-app-text",
              )}>
              {template.template_name}
            </span>
            <span
              className={cn(
                "px-1.5 py-0.5 rounded text-[9px] font-mono",
                isSelected
                  ? "bg-emerald-500/20 text-emerald-400"
                  : "bg-app-card/50 text-app-subtext",
              )}>
              {Math.round(template.score * 100)}%
            </span>
          </div>
          {template.example_question && (
            <p className="text-[10px] text-app-subtext mt-1 ml-5 italic">
              Example: "{template.example_question}"
            </p>
          )}
        </div>
        {!isSelected && (
          <span className="text-[9px] text-primary font-medium shrink-0">
            Use this
          </span>
        )}
        {isSelected && (
          <span className="text-[9px] text-emerald-400 font-medium shrink-0">
            Selected
          </span>
        )}
      </div>
    </div>
  );
}
