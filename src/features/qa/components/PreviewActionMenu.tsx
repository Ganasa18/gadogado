import { useState } from "react";
import {
  AlertTriangle,
  Camera,
  ChevronLeft,
  Maximize2,
  PauseCircle,
  PlayCircle,
  Plus,
  RefreshCcw,
} from "lucide-react";
import type { CaptureMode, RecordingMode, ScreenshotMode } from "../../../store/qaSession";

type PreviewActionMenuProps = {
  isRecording: boolean;
  recordingMode: RecordingMode;
  screenshotMode: ScreenshotMode;
  captureMode: CaptureMode;
  recordingDelay: number;
  isRecordingArmed: boolean;
  canStartRecording: boolean;
  canStopRecording: boolean;
  canEndSession: boolean;
  previewUrlValid: boolean;
  screenshotLoading: boolean;
  onRecordingModeChange: (mode: RecordingMode) => void;
  onScreenshotModeChange: (mode: ScreenshotMode) => void;
  onCaptureModeChange: (mode: CaptureMode) => void;
  onRecordingDelayChange: (value: number) => void;
  onManualRecordNext: () => void;
  onStartRecording: () => void;
  onStopRecording: () => void;
  onEndSession: () => void;
  onCaptureScreenshot: () => void;
  onReloadPreview: () => void;
  onBackPreview: () => void;
  onToggleFullscreen: () => void;
};

export default function PreviewActionMenu({
  isRecording,
  recordingMode,
  screenshotMode,
  captureMode,
  recordingDelay,
  isRecordingArmed,
  canStartRecording,
  canStopRecording,
  canEndSession,
  previewUrlValid,
  screenshotLoading,
  onRecordingModeChange,
  onScreenshotModeChange,
  onCaptureModeChange,
  onRecordingDelayChange,
  onManualRecordNext,
  onStartRecording,
  onStopRecording,
  onEndSession,
  onCaptureScreenshot,
  onReloadPreview,
  onBackPreview,
  onToggleFullscreen,
}: PreviewActionMenuProps) {
  const [fabOpen, setFabOpen] = useState(false);

  return (
    <div className="absolute top-2 right-2 z-20" data-qa-record-ignore>
      <div className="relative flex items-start justify-end">
        <button
          type="button"
          onClick={() => setFabOpen((open) => !open)}
          aria-label={fabOpen ? "Close action menu" : "Open action menu"}
          aria-expanded={fabOpen}
          className="flex h-9 w-9 items-center justify-center rounded-full border border-emerald-500/40 bg-emerald-900/60 text-emerald-100 shadow-md shadow-emerald-900/20 transition hover:border-emerald-400/70 hover:bg-emerald-900/50">
          <Plus
            className={`h-4 w-4 transition-transform duration-200 ${
              fabOpen ? "rotate-45" : "rotate-0"
            }`}
          />
        </button>

        <div
          className={`absolute right-0 top-full mt-2 flex flex-col gap-2 transition-all duration-200 ease-out ${
            fabOpen
              ? "pointer-events-auto translate-y-0 opacity-100"
              : "pointer-events-none -translate-y-2 opacity-0"
          }`}>
          {!isRecording && (
            <div className="flex items-center gap-1 bg-[#151c1b] border border-app-border rounded px-2 py-1">
              <button
                type="button"
                onClick={() => onRecordingModeChange("auto")}
                className={`px-2 py-0.5 text-[9px] rounded transition ${
                  recordingMode === "auto"
                    ? "bg-emerald-700/40 text-emerald-100 border border-emerald-500/50"
                    : "text-app-subtext hover:text-app-text"
                }`}>
                Auto
              </button>
              <button
                type="button"
                onClick={() => onRecordingModeChange("manual")}
                className={`px-2 py-0.5 text-[9px] rounded transition ${
                  recordingMode === "manual"
                    ? "bg-blue-700/40 text-blue-100 border border-blue-500/50"
                    : "text-app-subtext hover:text-app-text"
                }`}>
                Manual
              </button>
            </div>
          )}

          {!isRecording && (
            <div className="flex items-center gap-1 bg-[#151c1b] border border-app-border rounded px-2 py-1">
              <label className="text-[9px] text-app-subtext">Shots:</label>
              <button
                type="button"
                onClick={() => onScreenshotModeChange("auto")}
                className={`px-2 py-0.5 text-[9px] rounded transition ${
                  screenshotMode === "auto"
                    ? "bg-emerald-700/40 text-emerald-100 border border-emerald-500/50"
                    : "text-app-subtext hover:text-app-text"
                }`}>
                Auto
              </button>
              <button
                type="button"
                onClick={() => onScreenshotModeChange("manual")}
                className={`px-2 py-0.5 text-[9px] rounded transition ${
                  screenshotMode === "manual"
                    ? "bg-blue-700/40 text-blue-100 border border-blue-500/50"
                    : "text-app-subtext hover:text-app-text"
                }`}>
                Manual
              </button>
            </div>
          )}

          {!isRecording && (
            <div className="flex items-center gap-1 bg-[#151c1b] border border-app-border rounded px-2 py-1">
              <label className="text-[9px] text-app-subtext">Scope:</label>
              <button
                type="button"
                onClick={() => onCaptureModeChange("windowed_frame")}
                className={`px-2 py-0.5 text-[9px] rounded transition ${
                  captureMode === "windowed_frame"
                    ? "bg-emerald-700/40 text-emerald-100 border border-emerald-500/50"
                    : "text-app-subtext hover:text-app-text"
                }`}>
                Window
              </button>
              <button
                type="button"
                onClick={() => onCaptureModeChange("full_screen")}
                className={`px-2 py-0.5 text-[9px] rounded transition ${
                  captureMode === "full_screen"
                    ? "bg-blue-700/40 text-blue-100 border border-blue-500/50"
                    : "text-app-subtext hover:text-app-text"
                }`}>
                Full
              </button>
            </div>
          )}

          {!isRecording && (
            <div className="flex items-center gap-1 bg-[#151c1b] border border-app-border rounded px-2 py-1">
              <label htmlFor="recording-delay" className="text-[9px] text-app-subtext">
                Delay:
              </label>
              <input
                id="recording-delay"
                type="number"
                min="0"
                max="5000"
                step="100"
                value={recordingDelay}
                onChange={(e) => onRecordingDelayChange(Number(e.target.value))}
                className="w-12 bg-black/30 border border-app-border rounded px-1 py-0.5 text-[9px] text-app-text focus:outline-none focus:border-emerald-500/50"
              />
              <span className="text-[9px] text-app-subtext">ms</span>
            </div>
          )}

          {isRecording && recordingMode === "manual" && (
            <button
              type="button"
              onClick={onManualRecordNext}
              className={`flex items-center gap-2 border rounded px-2 py-1 text-[10px] transition ${
                isRecordingArmed
                  ? "bg-blue-900/40 border-blue-500/60 text-blue-100"
                  : "bg-[#1a2a3a] border-blue-800/40 text-blue-200 hover:border-blue-500/60"
              }`}>
              <PlayCircle className="w-3 h-3" />
              {isRecordingArmed ? "Armed (Cancel)" : "Record Next"}
            </button>
          )}

          <button
            type="button"
            onClick={onStartRecording}
            disabled={!canStartRecording}
            className="flex items-center gap-2 bg-[#133122] border border-emerald-800/40 rounded px-2 py-1 text-[10px] text-emerald-200 hover:border-emerald-500/60 transition disabled:opacity-50">
            <PlayCircle className="w-3 h-3" />
            {isRecording ? "Recording..." : "Start Record"}
          </button>
          <button
            type="button"
            onClick={onStopRecording}
            disabled={!canStopRecording}
            className="flex items-center gap-2 bg-[#2a1d1d] border border-red-900/40 rounded px-2 py-1 text-[10px] text-red-200 hover:border-red-700/60 transition disabled:opacity-50">
            <PauseCircle className="w-3 h-3" />
            Stop Record
          </button>
          <button
            type="button"
            onClick={onEndSession}
            disabled={!canEndSession}
            className="flex items-center gap-2 bg-[#2a1414] border border-red-800/50 rounded px-2 py-1 text-[10px] text-red-200 hover:border-red-500/70 hover:text-red-100 transition disabled:opacity-50 shadow-sm">
            <AlertTriangle className="w-3 h-3" />
            End Session
          </button>
        </div>

        <div
          className={`absolute right-full top-0 mr-2 flex items-center gap-2 transition-all duration-200 ease-out ${
            fabOpen
              ? "pointer-events-auto translate-x-0 opacity-100"
              : "pointer-events-none translate-x-2 opacity-0"
          }`}>
          <button
            type="button"
            onClick={onCaptureScreenshot}
            disabled={screenshotLoading}
            className="flex items-center gap-2 bg-[#151c1b] border border-app-border rounded px-2 py-1 text-[10px] text-app-subtext hover:text-app-text hover:border-emerald-600/60 transition disabled:opacity-50">
            <Camera className="w-3 h-3" />
            Capture
          </button>
          <button
            type="button"
            onClick={onReloadPreview}
            disabled={!previewUrlValid}
            className="flex items-center gap-2 bg-[#151c1b] border border-app-border rounded px-2 py-1 text-[10px] text-app-subtext hover:text-app-text hover:border-emerald-600/60 transition disabled:opacity-50">
            <RefreshCcw className="w-3 h-3" />
            Reload
          </button>
          <button
            type="button"
            onClick={onBackPreview}
            disabled={!previewUrlValid}
            className="flex items-center gap-2 bg-[#151c1b] border border-app-border rounded px-2 py-1 text-[10px] text-app-subtext hover:text-app-text hover:border-emerald-600/60 transition disabled:opacity-50">
            <ChevronLeft className="w-3 h-3" />
            Back
          </button>
          <button
            type="button"
            onClick={onToggleFullscreen}
            disabled={!previewUrlValid}
            className="flex items-center gap-2 bg-[#151c1b] border border-app-border rounded px-2 py-1 text-[10px] text-app-subtext hover:text-app-text hover:border-emerald-600/60 transition disabled:opacity-50 justify-center">
            <Maximize2 className="w-3 h-3" />
            Full
          </button>
        </div>
      </div>
    </div>
  );
}
