import { useEffect, useMemo, useState } from "react";
import { ClipboardList, FolderClock, RefreshCcw, Search } from "lucide-react";
import { useNavigate } from "react-router";
import { invoke } from "@tauri-apps/api/core";
import { useToastStore } from "../../store/toast";
import { isTauri } from "../../utils/tauri";

interface QaSession {
  id: string;
  title: string;
  goal: string;
  is_positive_case: boolean;
  app_version?: string | null;
  os?: string | null;
  started_at: number;
  ended_at?: number | null;
  notes?: string | null;
}

const DEFAULT_LIMIT = 75;

export default function SessionHistoryTab() {
  const { addToast } = useToastStore();
  const navigate = useNavigate();
  const [sessions, setSessions] = useState<QaSession[]>([]);
  const [loadingSessions, setLoadingSessions] = useState(false);
  const [search, setSearch] = useState("");

  const filteredSessions = useMemo(() => {
    const term = search.trim().toLowerCase();
    if (!term) return sessions;
    return sessions.filter((session) => {
      return (
        session.title.toLowerCase().includes(term) ||
        session.goal.toLowerCase().includes(term) ||
        session.id.toLowerCase().includes(term)
      );
    });
  }, [search, sessions]);

  useEffect(() => {
    if (!isTauri()) return;
    void loadSessions();
  }, []);

  const loadSessions = async () => {
    if (!isTauri()) {
      addToast("QA history is only available in the Tauri app", "error");
      return;
    }
    setLoadingSessions(true);
    try {
      const data = await invoke<QaSession[]>("qa_list_sessions", {
        limit: DEFAULT_LIMIT,
      });
      setSessions(data);
    } catch (err) {
      console.error(err);
      addToast("Failed to load QA sessions", "error");
    } finally {
      setLoadingSessions(false);
    }
  };

  const handleOpenSession = (session: QaSession) => {
    navigate(`/qa/session/${session.id}`);
  };

  return (
    <div className="flex bg-app-bg text-app-text min-h-full overflow-hidden">
      <aside className="w-full overflow-y-auto p-4 flex flex-col gap-4">
        <div className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm">
          <div className="flex items-center justify-between gap-3">
            <div>
              <div className="flex items-center gap-2 text-app-text font-medium">
                <FolderClock className="w-4 h-4 text-emerald-400" />
                <h3>QA Session History</h3>
              </div>
              <p className="text-[11px] text-app-subtext mt-1">
                Review QA sessions and open the session detail view.
              </p>
            </div>
            <button
              type="button"
              onClick={loadSessions}
              className="flex items-center gap-2 bg-[#151c1b] border border-emerald-900/40 rounded px-3 py-2 text-[11px] text-emerald-200 hover:border-emerald-600/60 transition disabled:opacity-50"
              disabled={loadingSessions}>
              <RefreshCcw className="w-3.5 h-3.5" />
              {loadingSessions ? "Refreshing..." : "Refresh"}
            </button>
          </div>
        </div>

        <section className="bg-app-card rounded-lg border border-app-border p-4 shadow-sm">
          <div className="flex items-center gap-2 mb-3 text-app-text font-medium">
            <ClipboardList className="w-4 h-4 text-sky-300" />
            <h4>Sessions</h4>
          </div>
          <div className="relative mb-3">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-app-subtext" />
            <input
              className="w-full bg-[#181818] border border-app-border rounded p-2 pl-8 text-xs outline-none focus:border-gray-500 transition"
              placeholder="Search sessions..."
              value={search}
              onChange={(e) => setSearch(e.currentTarget.value)}
            />
          </div>
          <div className="space-y-2 max-h-[640px] overflow-y-auto pr-1">
            {filteredSessions.length === 0 && (
              <div className="border border-dashed border-app-border/70 rounded-md p-4 text-[11px] text-app-subtext">
                {loadingSessions ? "Loading sessions..." : "No QA sessions found."}
              </div>
            )}
            {filteredSessions.map((session) => (
              <button
                key={session.id}
                type="button"
                onClick={() => handleOpenSession(session)}
                className="w-full text-left rounded-md border border-app-border bg-black/20 hover:border-emerald-500/40 transition p-3">
                <div className="flex items-center justify-between gap-2">
                  <div className="text-xs font-semibold text-app-text">
                    {session.title || "Untitled Session"}
                  </div>
                  <div
                    className={`text-[10px] px-2 py-0.5 rounded-full ${
                      session.ended_at
                        ? "bg-[#2b2b2b] text-gray-300"
                        : "bg-[#173121] text-emerald-200"
                    }`}>
                    {session.ended_at ? "Ended" : "Active"}
                  </div>
                </div>
                <div className="text-[10px] text-app-subtext mt-1 line-clamp-2">
                  {session.goal}
                </div>
                <div className="text-[10px] text-app-subtext mt-2">
                  {formatTimestamp(session.started_at)}
                </div>
              </button>
            ))}
          </div>
        </section>
      </aside>
    </div>
  );
}

function formatTimestamp(timestamp: number) {
  return new Date(timestamp).toLocaleString();
}
