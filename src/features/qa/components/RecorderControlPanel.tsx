import type { QaSession } from "../../../types/qa/types";

type RecorderControlPanelProps = {
  session: QaSession | null;
  runId: string | null;
  isRecording: boolean;
  canStart: boolean;
  canStop: boolean;
  canEnd: boolean;
  targetUrl: string | null;
  screenshotDelay: number;
  recorderEventInterval: number;
  onScreenshotDelayChange: (value: number) => void;
  onRecorderEventIntervalChange: (value: number) => void;
  onStartManual: () => void;
  onStartAiExplore: () => void;
  onStop: () => void;
  onEndSession: () => void;
  isExploring?: boolean;
};

export default function RecorderControlPanel({
  session,
  runId,
  isRecording,
  canStart,
  canStop,
  canEnd,
  targetUrl,
  screenshotDelay,
  recorderEventInterval,
  onScreenshotDelayChange,
  onRecorderEventIntervalChange,
  onStartManual,
  onStartAiExplore,
  onStop,
  onEndSession,
  isExploring = false,
}: RecorderControlPanelProps) {
  return (
    <section className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm space-y-4">
      <div className="flex items-center justify-between gap-3">
        <div>
          <div className="text-[11px] text-app-subtext uppercase tracking-wide">
            Browser Recorder
          </div>
          <div className="text-sm text-app-text font-semibold">External Playwright</div>
        </div>
        <div className="text-[10px] text-app-subtext">
          {isRecording ? "Recording" : "Idle"}
        </div>
      </div>

      <div className="rounded border border-app-border bg-black/20 px-3 py-2 text-[11px]">
        <div className="text-[10px] text-app-subtext">Target URL</div>
        <div className="text-app-text break-all">{targetUrl ?? "n/a"}</div>
      </div>

      <div className="grid grid-cols-2 gap-3 text-[11px]">
        <div className="rounded border border-app-border bg-black/20 px-3 py-2">
          <div className="text-[10px] text-app-subtext">Session</div>
          <div className="text-app-text">{session?.title ?? "n/a"}</div>
        </div>
        <div className="rounded border border-app-border bg-black/20 px-3 py-2">
          <div className="text-[10px] text-app-subtext">Run ID</div>
          <div className="text-app-text">{runId ? runId.slice(0, 8) : "n/a"}</div>
        </div>
      </div>

      <div className="rounded border border-app-border bg-black/20 px-3 py-2 text-[11px]">
        <div className="text-[10px] text-app-subtext">Screenshot Delay</div>
        <div className="mt-1 flex items-center gap-2">
          <input
            type="number"
            min="0"
            max="5000"
            step="100"
            value={screenshotDelay}
            onChange={(event) =>
              onScreenshotDelayChange(Number(event.target.value))
            }
            className="w-20 rounded border border-app-border bg-black/30 px-2 py-1 text-[11px] text-app-text focus:outline-none focus:border-emerald-500/50"
          />
          <span className="text-[10px] text-app-subtext">ms after each event</span>
        </div>
      </div>

      <div className="rounded border border-app-border bg-black/20 px-3 py-2 text-[11px]">
        <div className="text-[10px] text-app-subtext">Event Batch Window</div>
        <div className="mt-1 flex items-center gap-2">
          <input
            type="number"
            min="0"
            max="2000"
            step="50"
            value={recorderEventInterval}
            onChange={(event) =>
              onRecorderEventIntervalChange(Number(event.target.value))
            }
            className="w-20 rounded border border-app-border bg-black/30 px-2 py-1 text-[11px] text-app-text focus:outline-none focus:border-emerald-500/50"
          />
          <span className="text-[10px] text-app-subtext">ms per batch</span>
        </div>
      </div>

      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          onClick={onStartManual}
          disabled={!canStart}
          className="rounded border border-emerald-500/40 bg-emerald-500/10 px-3 py-2 text-[11px] text-emerald-200 hover:border-emerald-400/80 transition disabled:opacity-50">
          Start Manual
        </button>
        <button
          type="button"
          onClick={onStartAiExplore}
          disabled={!canStart || isExploring}
          className="rounded border border-sky-500/40 bg-sky-500/10 px-3 py-2 text-[11px] text-sky-200 hover:border-sky-400/80 transition disabled:opacity-50 flex items-center gap-2">
          {isExploring ? (
            <>
              <div className="w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin" />
              <span>Analyzing...</span>
            </>
          ) : (
            "AI Explore"
          )}
        </button>
        <button
          type="button"
          onClick={onStop}
          disabled={!canStop}
          className="rounded border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-[11px] text-amber-200 hover:border-amber-400/80 transition disabled:opacity-50">
          Stop Recorder
        </button>
        <button
          type="button"
          onClick={onEndSession}
          disabled={!canEnd}
          className="rounded border border-red-500/40 bg-red-500/10 px-3 py-2 text-[11px] text-red-200 hover:border-red-400/80 transition disabled:opacity-50">
          End Session
        </button>
      </div>

      <div className="text-[10px] text-app-subtext">
        Recorder opens in a separate browser window. Events and network data stream
        into this session automatically.
      </div>
    </section>
  );
}
