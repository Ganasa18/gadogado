import { useMemo, useState } from "react";
import { ClipboardCheck, Save } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { Switch } from "../../shared/components/Switch";
import { useToastStore } from "../../store/toast";
import { useQaSessionStore } from "../../store/qaSession";
import { isTauri } from "../../utils/tauri";
import { QaSession, QaSessionType } from "../../types/qa/types";

const DEFAULT_TITLE = "Untitled Session";

export default function SessionManagerTab() {
  const { addToast } = useToastStore();
  const { activeSessionId, setActiveSessionId, setRecordingSessionId } =
    useQaSessionStore();
  const [title, setTitle] = useState(DEFAULT_TITLE);
  const [goal, setGoal] = useState("");
  const [sessionType, setSessionType] = useState<QaSessionType>("browser");
  const [targetUrl, setTargetUrl] = useState("");
  const [apiBaseUrl, setApiBaseUrl] = useState("");
  const [authProfileJson, setAuthProfileJson] = useState("");
  const [isPositiveCase, setIsPositiveCase] = useState(true);
  const [activeSession, setActiveSession] = useState<QaSession | null>(null);
  const [isStarting, setIsStarting] = useState(false);

  const canSave =
    goal.trim().length > 0 &&
    !isStarting &&
    (sessionType === "browser" || apiBaseUrl.trim().length > 0);

  const startedAtLabel = useMemo(() => {
    if (!activeSession?.started_at) return "Not started";
    return new Date(activeSession.started_at).toLocaleString();
  }, [activeSession]);

  const handleSave = async () => {
    if (!isTauri()) {
      addToast("Tauri runtime not available in browser mode", "error");
      return;
    }
    if (!goal.trim()) {
      addToast("Goal is required to save a session", "error");
      return;
    }

    if (sessionType === "api" && !apiBaseUrl.trim()) {
      addToast("API base URL is required for API sessions", "error");
      return;
    }

    setIsStarting(true);
    try {
      const session = await invoke<QaSession>("qa_start_session", {
        title: title.trim(),
        goal: goal.trim(),
        sessionType,
        isPositiveCase,
        targetUrl: sessionType === "browser" ? targetUrl.trim() : null,
        apiBaseUrl: sessionType === "api" ? apiBaseUrl.trim() : null,
        authProfileJson: sessionType === "api" ? authProfileJson.trim() : null,
        notes: null,
      });
      setActiveSessionId(session.id);
      setRecordingSessionId(null);
      setActiveSession(session);
      addToast("QA session saved", "success");
    } catch (err) {
      console.error(err);
      addToast("Failed to save QA session", "error");
    } finally {
      setIsStarting(false);
    }
  };

  return (
    <div className="flex bg-app-bg text-app-text min-h-full overflow-hidden">
      <aside className="w-full overflow-y-auto p-4 flex flex-col gap-4">
        <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm">
          <div className="flex items-center gap-2 mb-3 text-app-text font-medium">
            <ClipboardCheck className="w-4 h-4 text-emerald-400" />
            <h3>Session Manager</h3>
          </div>
          <div className="space-y-3">
            <div>
              <label className="text-[10px] text-gray-500 block mb-1">
                Title
              </label>
              <input
                className="w-full bg-[#181818] border border-app-border rounded p-2 px-3 text-xs outline-none focus:border-gray-500 transition"
                placeholder={DEFAULT_TITLE}
                value={title}
                onChange={(e) => setTitle(e.currentTarget.value)}
              />
            </div>
            <div>
              <label className="text-[10px] text-gray-500 block mb-1">
                Goal (required)
              </label>
              <textarea
                className="w-full min-h-[96px] bg-[#181818] border border-app-border rounded p-2 px-3 text-xs outline-none focus:border-gray-500 transition resize-y"
                placeholder="Describe the QA goal for this recording session..."
                value={goal}
                onChange={(e) => setGoal(e.currentTarget.value)}
              />
            </div>
            <div>
              <label className="text-[10px] text-gray-500 block mb-1">
                Session type
              </label>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => setSessionType("browser")}
                  className={`px-3 py-1 rounded text-[11px] border transition ${
                    sessionType === "browser"
                      ? "bg-emerald-700/30 border-emerald-500/60 text-emerald-100"
                      : "bg-[#181818] border-app-border text-app-subtext"
                  }`}>
                  Browser
                </button>
                <button
                  type="button"
                  onClick={() => setSessionType("api")}
                  className={`px-3 py-1 rounded text-[11px] border transition ${
                    sessionType === "api"
                      ? "bg-blue-700/30 border-blue-500/60 text-blue-100"
                      : "bg-[#181818] border-app-border text-app-subtext"
                  }`}>
                  API
                </button>
              </div>
            </div>
            {sessionType === "browser" ? (
              <div>
                <label className="text-[10px] text-gray-500 block mb-1">
                  Target URL (optional)
                </label>
                <input
                  className="w-full bg-[#181818] border border-app-border rounded p-2 px-3 text-xs outline-none focus:border-gray-500 transition"
                  placeholder="https://app.example.com"
                  value={targetUrl}
                  onChange={(e) => setTargetUrl(e.currentTarget.value)}
                />
              </div>
            ) : (
              <div className="space-y-3">
                <div>
                  <label className="text-[10px] text-gray-500 block mb-1">
                    API base URL (required)
                  </label>
                  <input
                    className="w-full bg-[#181818] border border-app-border rounded p-2 px-3 text-xs outline-none focus:border-gray-500 transition"
                    placeholder="https://api.example.com"
                    value={apiBaseUrl}
                    onChange={(e) => setApiBaseUrl(e.currentTarget.value)}
                  />
                </div>
                <div>
                  <label className="text-[10px] text-gray-500 block mb-1">
                    Auth profile JSON (optional)
                  </label>
                  <textarea
                    className="w-full min-h-[72px] bg-[#181818] border border-app-border rounded p-2 px-3 text-xs outline-none focus:border-gray-500 transition resize-y"
                    placeholder='{"Authorization":"Bearer <token>"}'
                    value={authProfileJson}
                    onChange={(e) => setAuthProfileJson(e.currentTarget.value)}
                  />
                </div>
              </div>
            )}
            <div className="flex items-center justify-between">
              <div>
                <div className="text-xs text-gray-200 font-medium">
                  Positive case
                </div>
                <div className="text-[10px] text-gray-500">
                  {sessionType === "api"
                    ? "Optional for API sessions; keep for coverage grouping."
                    : "Toggle off for negative or edge-case recordings."}
                </div>
              </div>
              <Switch
                checked={isPositiveCase}
                onCheckedChange={setIsPositiveCase}
              />
            </div>
          </div>
          <div className="mt-4">
            <button
              className="flex w-full items-center justify-center gap-2 bg-[#133122] border border-emerald-800/40 rounded p-2 text-xs text-emerald-200 hover:border-emerald-500/60 transition disabled:opacity-50 disabled:cursor-not-allowed"
              disabled={!canSave}
              onClick={handleSave}>
              <Save className="w-3.5 h-3.5" />
              {isStarting ? "Saving..." : "Save Session"}
            </button>
          </div>
          {!canSave && goal.trim().length === 0 && (
            <div className="mt-3 text-[10px] text-amber-300">
              Add a goal to save the session.
            </div>
          )}
          {activeSessionId && (
            <div className="mt-3 text-[10px] text-emerald-300">
              Latest session ID: {activeSessionId}
            </div>
          )}
        </div>

        <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm">
          <div className="flex items-center gap-2 mb-3 text-app-text font-medium">
            <span className="text-xs uppercase tracking-wider text-app-subtext/70">
              Status
            </span>
          </div>
          <div className="grid grid-cols-2 gap-3 text-xs">
            <div className="rounded-md border border-app-border bg-black/20 p-3">
              <div className="text-[10px] text-gray-500">State</div>
              <div
                className={
                  activeSessionId ? "text-emerald-300" : "text-gray-300"
                }>
                {activeSessionId ? "Saved" : "Idle"}
              </div>
            </div>
            <div className="rounded-md border border-app-border bg-black/20 p-3">
              <div className="text-[10px] text-gray-500">Started at</div>
              <div className="text-gray-300">{startedAtLabel}</div>
            </div>
          </div>
          {activeSession && (
            <div className="mt-3 text-[10px] text-gray-500">
              Goal: {activeSession.goal}
            </div>
          )}
        </div>
      </aside>
    </div>
  );
}
