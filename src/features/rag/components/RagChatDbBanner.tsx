import { Database, Shield } from "lucide-react";
import { useNavigate } from "react-router";

export function RagChatDbBanner(props: {
  isDbCollection: boolean;
  selectedCollectionId: number | null;
  selectedTables: string[];
}) {
  const { isDbCollection, selectedCollectionId, selectedTables } = props;
  const navigate = useNavigate();

  if (!isDbCollection || !selectedCollectionId) return null;

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
