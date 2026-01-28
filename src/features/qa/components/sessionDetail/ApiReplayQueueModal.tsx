import { X } from "lucide-react";

type ApiReplayItem = {
  id: string;
  requestLabel: string;
  responseLabel?: string;
  status: "pending" | "running" | "success" | "error";
};

type ApiReplayQueueModalProps = {
  items: ApiReplayItem[];
  onClose: () => void;
};

export function ApiReplayQueueModal({
  items,
  onClose,
}: ApiReplayQueueModalProps) {
  return (
    <div className="fixed inset-0 z-[10000] flex items-center justify-center bg-black/70 backdrop-blur-sm p-4">
      <div className="w-full max-w-4xl bg-app-panel border border-app-border rounded-2xl shadow-2xl p-5 animate-in zoom-in-95 duration-200">
        <div className="flex items-center justify-between gap-3 mb-4">
          <div>
            <div className="text-xs uppercase tracking-widest text-app-subtext">
              API Replay Queue
            </div>
            <div className="text-sm font-semibold text-app-text">
              Request + Response Timeline
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-full border border-app-border p-2 text-app-subtext hover:text-app-text hover:border-emerald-500/60 transition">
            <X className="w-4 h-4" />
          </button>
        </div>
        <div className="max-h-[500px] overflow-y-auto space-y-2 pr-1 custom-scrollbar">
          {items.map((item, index) => (
            <div
              key={item.id}
              className={`rounded-lg border px-3 py-2.5 text-xs transition-colors ${
                item.status === "success"
                  ? "border-emerald-500/30 bg-emerald-500/10"
                  : item.status === "error"
                    ? "border-red-500/30 bg-red-500/10"
                    : item.status === "running"
                      ? "border-sky-500/30 bg-sky-500/10"
                      : "border-app-border bg-black/20"
              }`}>
              <div className="flex items-center justify-between gap-2 mb-1">
                <span className="text-[10px] uppercase tracking-wide opacity-60">
                  Step {index + 1}
                </span>
                <span
                  className={`text-[10px] uppercase font-bold tracking-wider ${
                    item.status === "success"
                      ? "text-emerald-400"
                      : item.status === "error"
                        ? "text-red-400"
                        : item.status === "running"
                          ? "text-sky-400"
                          : "text-app-subtext"
                  }`}>
                  {item.status}
                </span>
              </div>
              <div className="font-mono text-app-text opacity-90 truncate">
                {item.requestLabel}
              </div>
              {item.responseLabel && (
                <div className="mt-1 text-app-subtext text-[11px] border-t border-dashed border-white/10 pt-1">
                  {item.responseLabel}
                </div>
              )}
            </div>
          ))}
          {items.length === 0 && (
            <div className="text-xs text-app-subtext text-center py-8">
              No API events queued for replay.
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
