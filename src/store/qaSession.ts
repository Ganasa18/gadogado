import { create } from "zustand";

export type RecordingMode = "auto" | "manual";

interface QaSessionState {
  activeSessionId: string | null;
  recordingSessionId: string | null;
  recordingMode: RecordingMode;
  recordingDelay: number; // in milliseconds
  isRecordingArmed: boolean;
  setActiveSessionId: (id: string | null) => void;
  setRecordingSessionId: (id: string | null) => void;
  setRecordingMode: (mode: RecordingMode) => void;
  setRecordingDelay: (delay: number) => void;
  setIsRecordingArmed: (armed: boolean) => void;
}

export const useQaSessionStore = create<QaSessionState>((set) => ({
  activeSessionId: null,
  recordingSessionId: null,
  recordingMode: "auto",
  recordingDelay: 500, // Default 500ms delay
  isRecordingArmed: false,
  setActiveSessionId: (activeSessionId) => set({ activeSessionId }),
  setRecordingSessionId: (recordingSessionId) => set({ recordingSessionId }),
  setRecordingMode: (recordingMode) => set({ recordingMode }),
  setRecordingDelay: (recordingDelay) => set({ recordingDelay }),
  setIsRecordingArmed: (isRecordingArmed) => set({ isRecordingArmed }),
}));
