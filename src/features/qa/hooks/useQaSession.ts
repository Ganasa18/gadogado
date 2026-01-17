import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { QaSession } from "../../../types/qa/types";

type UseQaSessionOptions = {
  sessionId: string;
  isTauriApp: boolean;
};

export default function useQaSession({
  sessionId,
  isTauriApp,
}: UseQaSessionOptions) {
  const [session, setSession] = useState<QaSession | null>(null);
  const [sessionLoading, setSessionLoading] = useState(true);
  const [sessionError, setSessionError] = useState<string | null>(null);

  const loadSession = async () => {
    if (!sessionId) {
      setSessionError("Missing session ID.");
      setSessionLoading(false);
      return;
    }
    if (!isTauriApp) {
      setSessionError("QA sessions are only available in the Tauri app.");
      setSessionLoading(false);
      return;
    }

    setSessionLoading(true);
    setSessionError(null);
    setSession(null);
    try {
      const data = await invoke<QaSession>("qa_get_session", {
        sessionId,
      });
      setSession(data);
    } catch (err) {
      console.error(err);
      setSessionError("Failed to load QA session.");
    } finally {
      setSessionLoading(false);
    }
  };

  useEffect(() => {
    void loadSession();
  }, [sessionId]);

  return {
    session,
    setSession,
    sessionLoading,
    sessionError,
    loadSession,
  };
}
