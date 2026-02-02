import { ChevronRight, Cpu, Settings2 } from "lucide-react";
import type { QaSession } from "../../../../types/qa/types";
import RecorderControlPanel from "../RecorderControlPanel";

type SessionControlsCardProps = {
  provider: string;
  model: string;
  modelOptions: string[];
  isLocalProvider: boolean;
  onModelChange: (value: string) => void;

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
  isExploring: boolean;
};

export function SessionControlsCard({
  provider,
  model,
  modelOptions,
  isLocalProvider,
  onModelChange,
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
  isExploring,
}: SessionControlsCardProps) {
  return (
    <div className="bg-app-card rounded-xl border border-app-border overflow-hidden shadow-sm">
      <div className="p-3 bg-app-panel/50 border-b border-app-border flex items-center justify-between">
        <div className="flex items-center gap-2 text-xs font-semibold text-app-text uppercase tracking-wide">
          <Settings2 className="w-3.5 h-3.5" />
          <span>Session Controls</span>
        </div>
      </div>
      <div className="p-4 space-y-5">
        <div className="space-y-2">
          <div className="flex items-center justify-between text-[11px] text-app-subtext">
            <span className="flex items-center gap-1.5">
              <Cpu className="w-3 h-3" /> Model Provider
            </span>
            <span className="uppercase">{provider}</span>
          </div>
          <div className="relative">
            <select
              value={model}
              onChange={(e) => onModelChange(e.target.value)}
              className="w-full bg-app-panel border border-app-border rounded-lg py-2 pl-3 pr-8 text-xs appearance-none focus:border-emerald-500/50 transition outline-none text-app-text">
              {modelOptions.map((opt) => (
                <option key={opt} value={opt} className="bg-app-panel text-app-text">
                  {opt}
                </option>
              ))}
            </select>
            <div className="absolute right-3 top-2.5 pointer-events-none text-app-subtext">
              <ChevronRight className="w-3.5 h-3.5 rotate-90" />
            </div>
          </div>
          {isLocalProvider && modelOptions[0] === "No models found" && (
            <div className="text-[10px] text-amber-500/80 bg-amber-500/10 px-2 py-1.5 rounded">
              No local models detected. Check server.
            </div>
          )}
        </div>

        <div className="h-px bg-app-border/50" />

        <RecorderControlPanel
          session={session}
          runId={runId}
          isRecording={isRecording}
          canStart={canStart}
          canStop={canStop}
          canEnd={canEnd}
          targetUrl={targetUrl}
          screenshotDelay={screenshotDelay}
          recorderEventInterval={recorderEventInterval}
          onScreenshotDelayChange={onScreenshotDelayChange}
          onRecorderEventIntervalChange={onRecorderEventIntervalChange}
          onStartManual={onStartManual}
          onStartAiExplore={onStartAiExplore}
          onStop={onStop}
          onEndSession={onEndSession}
          isExploring={isExploring}
        />
      </div>
    </div>
  );
}
