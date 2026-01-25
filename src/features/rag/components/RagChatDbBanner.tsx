import { useEffect, useState } from "react";
import { AlertTriangle, Database, Shield } from "lucide-react";
import { useNavigate } from "react-router";
import { dbGetRateLimitStatus } from "../api";
import type { RateLimitStatus } from "../types";

export function RagChatDbBanner(props: {
  isDbCollection: boolean;
  selectedCollectionId: number | null;
  selectedTables: string[];
}) {
  const { isDbCollection, selectedCollectionId, selectedTables } = props;
  const navigate = useNavigate();
  const [rateLimitStatus, setRateLimitStatus] = useState<RateLimitStatus | null>(null);

  // Fetch rate limit status when collection changes
  useEffect(() => {
    if (!isDbCollection || !selectedCollectionId) {
      setRateLimitStatus(null);
      return;
    }

    let cancelled = false;

    const fetchStatus = async () => {
      try {
        const status = await dbGetRateLimitStatus(selectedCollectionId);
        if (!cancelled) {
          setRateLimitStatus(status);
        }
      } catch (err) {
        console.error("Failed to fetch rate limit status:", err);
      }
    };

    void fetchStatus();

    // Refresh every 30 seconds
    const interval = setInterval(() => void fetchStatus(), 30000);

    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, [isDbCollection, selectedCollectionId]);

  if (!isDbCollection || !selectedCollectionId) return null;

  const rateLimitPercentage = rateLimitStatus
    ? Math.round((rateLimitStatus.queries_count / rateLimitStatus.max_queries_per_hour) * 100)
    : 0;
  const isNearLimit = rateLimitPercentage >= 80;
  const isAtLimit = rateLimitStatus?.is_rate_limited || rateLimitStatus?.is_cooldown_active;

  return (
    <div className="mx-6 mt-4 p-4 bg-purple-500/10 border border-purple-500/30 rounded-lg">
      <div className="flex items-start gap-3">
        <Database className="w-5 h-5 text-purple-500 mt-0.5 shrink-0" />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <h3 className="text-sm font-semibold text-purple-400">Database Query Mode</h3>
            <Shield className="w-4 h-4 text-purple-500" />
          </div>
          <p className="text-xs text-purple-300">
            This collection queries database tables. Queries are restricted to allowlisted tables only.
          </p>

          {selectedTables.length > 0 && (
            <div className="mt-2 flex flex-wrap items-center gap-2">
              <span className="text-[10px] text-purple-400">Selected tables:</span>
              <div className="flex flex-wrap gap-1">
                {selectedTables.map((table) => (
                  <span
                    key={table}
                    className="px-2 py-0.5 bg-purple-500/20 border border-purple-500/30 rounded text-[10px] text-purple-300">
                    {table}
                  </span>
                ))}
              </div>
            </div>
          )}

          {/* Rate Limit Status */}
          {rateLimitStatus && (
            <div className="mt-3 pt-2 border-t border-purple-500/20">
              <div className="flex items-center gap-3">
                <div className="flex items-center gap-2">
                  <span className="text-[10px] text-purple-400">Queries:</span>
                  <span
                    className={`text-[10px] font-medium ${isAtLimit ? "text-red-400" : isNearLimit ? "text-amber-400" : "text-purple-300"}`}>
                    {rateLimitStatus.queries_count} / {rateLimitStatus.max_queries_per_hour}
                  </span>
                </div>

                {/* Rate Limit Warning */}
                {isAtLimit && (
                  <div className="flex items-center gap-1.5 px-2 py-0.5 bg-red-500/10 border border-red-500/20 rounded">
                    <AlertTriangle className="w-3 h-3 text-red-400" />
                    <span className="text-[10px] text-red-400 font-medium">
                      {rateLimitStatus.is_cooldown_active
                        ? `Cooldown: ${Math.ceil((rateLimitStatus.retry_after_seconds ?? 0) / 60)}m`
                        : "Rate Limited"}
                    </span>
                  </div>
                )}

                {isNearLimit && !isAtLimit && (
                  <div className="flex items-center gap-1.5 px-2 py-0.5 bg-amber-500/10 border border-amber-500/20 rounded">
                    <AlertTriangle className="w-3 h-3 text-amber-400" />
                    <span className="text-[10px] text-amber-400 font-medium">Near Limit</span>
                  </div>
                )}
              </div>

              {/* Progress bar */}
              <div className="mt-1.5 h-1 bg-purple-500/20 rounded-full overflow-hidden">
                <div
                  className={`h-full transition-all duration-300 ${isAtLimit ? "bg-red-500" : isNearLimit ? "bg-amber-500" : "bg-purple-500"}`}
                  style={{ width: `${Math.min(100, rateLimitPercentage)}%` }}
                />
              </div>
            </div>
          )}

          <div className="mt-2 flex items-center gap-2">
            <button
              onClick={() => navigate("/database")}
              className="text-[10px] text-purple-400 hover:text-purple-300 underline">
              Configure connections {"->"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
