import { useRef } from "react";
import type { RefObject } from "react";
import { RefreshCcw, ScreenShare } from "lucide-react";
import type { CaptureMode, RecordingMode, ScreenshotMode } from "../../../store/qaSession";
import PreviewActionMenu from "./PreviewActionMenu";

type PreviewPanelProps = {
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
  previewLoading: boolean;
  previewError: string | null;
  previewReloadToken: number;
  proxiedPreviewUrl: string | null;
  showLivePreviewLoading: boolean;
  screenshotLoading: boolean;
  sessionLoading: boolean;
  previewFrameRef: RefObject<HTMLIFrameElement | null>;
  onFrameLoad: () => void;
  onFrameError: () => void;
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
};

export default function PreviewPanel({
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
  previewLoading,
  previewError,
  previewReloadToken,
  proxiedPreviewUrl,
  showLivePreviewLoading,
  screenshotLoading,
  sessionLoading,
  previewFrameRef,
  onFrameLoad,
  onFrameError,
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
}: PreviewPanelProps) {
  const previewContainerRef = useRef<HTMLDivElement | null>(null);

  const handleToggleFullscreen = () => {
    if (!previewContainerRef.current) return;
    if (document.fullscreenElement) {
      void document.exitFullscreen();
    } else {
      void previewContainerRef.current.requestFullscreen();
    }
  };

  return (
    <section className="space-y-4 col-span-2">
      <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm h-full flex flex-col">
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-2 text-app-text font-medium">
            <ScreenShare className="w-4 h-4 text-sky-300" />
            <h4>Preview</h4>
          </div>
          <div className="text-[10px] text-app-subtext">
            {isRecording ? "Live capture" : "Idle"}
          </div>
        </div>
        <div
          ref={previewContainerRef}
          data-qa-record-root
          className="mt-3 rounded-md border border-app-border bg-black/30 h-full overflow-hidden flex items-center justify-center relative">
            <PreviewActionMenu
              isRecording={isRecording}
              recordingMode={recordingMode}
              screenshotMode={screenshotMode}
              captureMode={captureMode}
              recordingDelay={recordingDelay}
              isRecordingArmed={isRecordingArmed}
              canStartRecording={canStartRecording}
              canStopRecording={canStopRecording}
              canEndSession={canEndSession}
              previewUrlValid={previewUrlValid}
              screenshotLoading={screenshotLoading}
              onRecordingModeChange={onRecordingModeChange}
              onScreenshotModeChange={onScreenshotModeChange}
              onCaptureModeChange={onCaptureModeChange}
              onRecordingDelayChange={onRecordingDelayChange}
              onManualRecordNext={onManualRecordNext}
              onStartRecording={onStartRecording}
              onStopRecording={onStopRecording}
              onEndSession={onEndSession}
              onCaptureScreenshot={onCaptureScreenshot}
              onReloadPreview={onReloadPreview}
              onBackPreview={onBackPreview}
              onToggleFullscreen={handleToggleFullscreen}
            />
          {isRecording && (
            <div className="absolute top-2 left-2 z-20 flex items-center gap-2 rounded-full border border-red-900/60 bg-red-900/20 px-2 py-0.5 text-[10px] text-red-100">
              <span className="h-2 w-2 rounded-full bg-red-400" />
              REC
            </div>
          )}
          {sessionLoading && (
            <div className="text-[11px] text-app-subtext">
              Loading preview...
            </div>
          )}
          {!sessionLoading && showLivePreviewLoading && (
            <div className="text-[11px] text-app-subtext text-center px-6">
              Capturing screenshot...
            </div>
          )}
          {!sessionLoading && !previewUrlValid && (
            <div className="text-[11px] text-app-subtext text-center px-6">
              Preview URL is missing or invalid. Add a valid URL when creating
              the session to enable the preview.
            </div>
          )}
          {!sessionLoading && previewUrlValid && (
            <iframe
              key={`${proxiedPreviewUrl}-${previewReloadToken}`}
              src={proxiedPreviewUrl ?? undefined}
              title="QA session preview"
              data-qa-preview-frame
              ref={previewFrameRef}
              className="w-full h-full border-none"
              onLoad={onFrameLoad}
              onError={onFrameError}
            />
          )}
        </div>
        {!sessionLoading && previewUrlValid && previewLoading && (
          <div className="mt-2 text-[10px] text-app-subtext">
            Loading preview...
          </div>
        )}
        {!sessionLoading && previewUrlValid && previewError && (
          <div className="mt-2 flex items-center gap-2 text-[11px] text-red-200">
            <span>{previewError}</span>
            <button
              type="button"
              onClick={onReloadPreview}
              className="flex items-center gap-2 bg-red-950/40 border border-red-900/50 rounded px-2 py-1 text-[10px] text-red-100 hover:border-red-600/60 transition">
              <RefreshCcw className="w-3 h-3" />
              Reload Preview
            </button>
          </div>
        )}
      </div>
    </section>
  );
}
