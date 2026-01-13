import { create } from "zustand";

export type RecordingMode = "auto" | "manual";
export type ScreenshotMode = "auto" | "manual";
export type CaptureMode = "windowed_frame" | "full_screen";

interface QaSessionState {
  activeSessionId: string | null;
  activeRunId: string | null;
  recordingSessionId: string | null;
  recordingMode: RecordingMode;
  screenshotMode: ScreenshotMode;
  captureMode: CaptureMode;
  recordingDelay: number; // in milliseconds
  screenshotDelay: number; // in milliseconds
  recorderEventInterval: number; // in milliseconds
  isRecordingArmed: boolean;
  setActiveSessionId: (id: string | null) => void;
  setActiveRunId: (id: string | null) => void;
  setRecordingSessionId: (id: string | null) => void;
  setRecordingMode: (mode: RecordingMode) => void;
  setScreenshotMode: (mode: ScreenshotMode) => void;
  setCaptureMode: (mode: CaptureMode) => void;
  setRecordingDelay: (delay: number) => void;
  setScreenshotDelay: (delay: number) => void;
  setRecorderEventInterval: (delay: number) => void;
  setIsRecordingArmed: (armed: boolean) => void;
}

export const useQaSessionStore = create<QaSessionState>((set) => ({
  activeSessionId: null,
  activeRunId: null,
  recordingSessionId: null,
  recordingMode: "auto",
  screenshotMode: "auto",
  captureMode: "windowed_frame",
  recordingDelay: 500, // Default 500ms delay
  screenshotDelay: 500,
  recorderEventInterval: 250,
  isRecordingArmed: false,
  setActiveSessionId: (activeSessionId) => set({ activeSessionId }),
  setActiveRunId: (activeRunId) => set({ activeRunId }),
  setRecordingSessionId: (recordingSessionId) => set({ recordingSessionId }),
  setRecordingMode: (recordingMode) => set({ recordingMode }),
  setScreenshotMode: (screenshotMode) => set({ screenshotMode }),
  setCaptureMode: (captureMode) => set({ captureMode }),
  setRecordingDelay: (recordingDelay) => set({ recordingDelay }),
  setScreenshotDelay: (screenshotDelay) => set({ screenshotDelay }),
  setRecorderEventInterval: (recorderEventInterval) =>
    set({ recorderEventInterval }),
  setIsRecordingArmed: (isRecordingArmed) => set({ isRecordingArmed }),
}));
