import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";
import { Plus, Database, Loader2, ChevronRight, AlertCircle, CheckCircle2, Trash2, RefreshCw } from "lucide-react";
import type { DbConnection, DbAllowlistProfile, TableInfo } from "../../rag/types";

interface TableAccessModalProps {
  connection: DbConnection | null;
  onClose: () => void;
}

export function TableAccessModal({ connection, onClose }: TableAccessModalProps) {
  const [profiles, setProfiles] = useState<DbAllowlistProfile[]>([]);
  const [selectedProfileId, setSelectedProfileId] = useState<number>(1);
  const [allowedTables, setAllowedTables] = useState<Set<string>>(new Set());
  const [availableTables, setAvailableTables] = useState<TableInfo[]>([]);
  const [newTableName, setNewTableName] = useState("");
  const [loading, setLoading] = useState(false);
  const [loadingTables, setLoadingTables] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  useEffect(() => {
    if (connection) {
      loadProfiles();
      loadAvailableTables();
    }
  }, [connection]);

  const loadProfiles = async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<DbAllowlistProfile[]>("db_list_allowlist_profiles");
      setProfiles(result);
      if (result.length > 0) {
        setSelectedProfileId(result[0].id);
        await loadAllowedTables(result[0].id);
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      setError(`Failed to load profiles: ${errorMsg}`);
    } finally {
      setLoading(false);
    }
  };

  const loadAllowedTables = async (profileId: number) => {
    try {
      await invoke<DbAllowlistProfile>("db_list_allowlisted_tables", { profileId });
      // Get the profile to parse its rules
      const profile = profiles.find(p => p.id === profileId);
      if (profile) {
        const parsedRules = JSON.parse(profile.rules_json);
        const tables = parsedRules.allowed_tables ? Object.keys(parsedRules.allowed_tables) : [];
        setAllowedTables(new Set(tables));
      }
    } catch (err) {
      console.error("Failed to load allowed tables:", err);
    }
  };

  const handleProfileChange = async (profileId: number) => {
    setSelectedProfileId(profileId);
    await loadAllowedTables(profileId);
  };

  const handleAddTable = () => {
    const trimmed = newTableName.trim();
    if (!trimmed) return;

    if (allowedTables.has(trimmed)) {
      setError(`Table "${trimmed}" is already in the allowlist`);
      return;
    }

    setAllowedTables(new Set([...allowedTables, trimmed]));
    setNewTableName("");
    setSuccess(`Table "${trimmed}" added to allowlist`);
    setTimeout(() => setSuccess(null), 3000);
  };

  const handleRemoveTable = (table: string) => {
    setAllowedTables(new Set([...allowedTables].filter(t => t !== table)));
  };

  const loadAvailableTables = async () => {
    if (!connection) return;

    setLoadingTables(true);
    setError(null);
    try {
      const tables = await invoke<TableInfo[]>("db_list_tables", {
        connId: connection.id
      });
      setAvailableTables(tables);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      setError(`Failed to load tables: ${errorMsg}`);
    } finally {
      setLoadingTables(false);
    }
  };

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4"
      onClick={onClose}
    >
      <motion.div
        initial={{ scale: 0.95, opacity: 0, y: 20 }}
        animate={{ scale: 1, opacity: 1, y: 0 }}
        exit={{ scale: 0.95, opacity: 0, y: 20 }}
        className="bg-app-panel border border-app-border rounded-xl shadow-2xl w-full max-w-2xl overflow-hidden flex flex-col max-h-[80vh]"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="px-6 py-4 border-b border-app-border flex justify-between items-center bg-app-card/30">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-lg bg-app-accent/10 flex items-center justify-center text-app-accent">
              <Database className="w-5 h-5" />
            </div>
            <div>
              <h2 className="text-lg font-semibold text-app-text">Manage Table Access</h2>
              <p className="text-xs text-app-subtext">{connection?.name || "Unknown Connection"}</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-2 hover:bg-app-card rounded-full text-app-subtext transition-colors"
          >
            <Plus className="w-5 h-5 rotate-45" />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 overflow-y-auto flex-1 space-y-6">
          {loading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="w-6 h-6 animate-spin text-app-accent" />
            </div>
          ) : (
            <>
              {/* Messages */}
              <AnimatePresence>
                {error && (
                  <motion.div
                    initial={{ opacity: 0, height: 0 }}
                    animate={{ opacity: 1, height: "auto" }}
                    exit={{ opacity: 0, height: 0 }}
                    className="bg-destructive/10 text-destructive border border-destructive/20 rounded-lg p-3 text-sm flex items-start gap-2"
                  >
                    <AlertCircle className="w-4 h-4 mt-0.5 shrink-0" />
                    <span>{error}</span>
                    <button
                      onClick={() => setError(null)}
                      className="ml-auto p-0.5 hover:bg-destructive/20 rounded"
                    >
                      <Plus className="w-3 h-3 rotate-45" />
                    </button>
                  </motion.div>
                )}
                {success && (
                  <motion.div
                    initial={{ opacity: 0, height: 0 }}
                    animate={{ opacity: 1, height: "auto" }}
                    exit={{ opacity: 0, height: 0 }}
                    className="bg-app-success/10 text-app-success border border-app-success/20 rounded-lg p-3 text-sm flex items-start gap-2"
                  >
                    <CheckCircle2 className="w-4 h-4 mt-0.5 shrink-0" />
                    <span>{success}</span>
                    <button
                      onClick={() => setSuccess(null)}
                      className="ml-auto p-0.5 hover:bg-app-success/20 rounded"
                    >
                      <Plus className="w-3 h-3 rotate-45" />
                    </button>
                  </motion.div>
                )}
              </AnimatePresence>

              {/* Profile Selector */}
              {profiles.length > 1 && (
                <div>
                  <label className="block text-xs font-bold uppercase tracking-wider text-app-subtext mb-2">
                    Profile
                  </label>
                  <div className="relative">
                    <select
                      value={selectedProfileId}
                      onChange={(e) => handleProfileChange(parseInt(e.target.value))}
                      className="w-full appearance-none bg-app-card border border-app-border rounded-lg px-4 py-2.5 text-app-text text-sm focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none transition-all cursor-pointer"
                    >
                      {profiles.map((profile) => (
                        <option key={profile.id} value={profile.id}>
                          {profile.name}
                        </option>
                      ))}
                    </select>
                    <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-app-subtext">
                      <ChevronRight className="w-3.5 h-3.5 rotate-90" />
                    </div>
                  </div>
                </div>
              )}

              {/* Add Table from Database */}
              <div>
                <div className="flex items-center justify-between mb-2">
                  <label className="block text-xs font-bold uppercase tracking-wider text-app-subtext">
                    Add Table to Allowlist
                  </label>
                  <button
                    onClick={loadAvailableTables}
                    disabled={loadingTables}
                    className="text-xs text-app-accent hover:text-app-accent/80 disabled:opacity-50 flex items-center gap-1 transition-colors"
                  >
                    <RefreshCw className={`w-3 h-3 ${loadingTables ? 'animate-spin' : ''}`} />
                    Refresh Tables
                  </button>
                </div>

                {loadingTables && (
                  <div className="flex items-center justify-center py-4">
                    <Loader2 className="w-5 h-5 animate-spin text-app-accent mr-2" />
                    <span className="text-sm text-app-subtext">Loading tables...</span>
                  </div>
                )}

                {!loadingTables && (
                  <>
                    <select
                      value={newTableName}
                      onChange={(e) => setNewTableName(e.target.value)}
                      disabled={availableTables.length === 0}
                      className="w-full appearance-none bg-app-card border border-app-border rounded-lg px-4 py-2.5 text-app-text text-sm focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none transition-all cursor-pointer disabled:opacity-50 mb-2"
                    >
                      <option value="">Select a table...</option>
                      {availableTables.map((table) => (
                        <option key={table.table_name} value={table.table_name}>
                          {table.table_name}
                          {table.table_schema ? ` (${table.table_schema})` : ''}
                          {table.row_count !== null ? ` [~${table.row_count} rows]` : ''}
                        </option>
                      ))}
                    </select>

                    <button
                      onClick={handleAddTable}
                      disabled={!newTableName.trim()}
                      className="w-full px-4 py-2 bg-app-accent text-white rounded-lg text-sm font-bold shadow-lg shadow-app-accent/20 hover:bg-app-accent/90 transition-all disabled:opacity-50 flex items-center justify-center gap-2"
                    >
                      <Plus className="w-4 h-4" />
                      Add Selected Table
                    </button>
                  </>
                )}
              </div>

              {/* Allowed Tables List */}
              <div>
                <label className="block text-xs font-bold uppercase tracking-wider text-app-subtext mb-2">
                  Allowed Tables ({allowedTables.size})
                </label>
                <div className="bg-app-card/50 border border-app-border rounded-lg p-4 min-h-[150px]">
                  {allowedTables.size === 0 ? (
                    <div className="text-center py-8">
                      <Database className="w-12 h-12 mx-auto text-app-subtext/40 mb-3" />
                      <p className="text-sm text-app-subtext">No tables in allowlist</p>
                      <p className="text-xs text-app-subtext mt-1">Add tables above to enable access</p>
                    </div>
                  ) : (
                    <div className="space-y-2">
                      {[...allowedTables].sort().map((table) => (
                        <div
                          key={table}
                          className="flex items-center justify-between p-3 bg-app-bg border border-app-border rounded-lg group hover:border-app-accent/50 transition-colors"
                        >
                          <div className="flex items-center gap-3">
                            <div className="w-8 h-8 rounded bg-app-accent/10 flex items-center justify-center text-app-accent">
                              <Database className="w-4 h-4" />
                            </div>
                            <code className="text-sm font-mono text-app-text">{table}</code>
                          </div>
                          <button
                            onClick={() => handleRemoveTable(table)}
                            className="p-2 text-app-subtext hover:text-destructive hover:bg-destructive/10 rounded transition-colors"
                            title="Remove table"
                          >
                            <Trash2 className="w-4 h-4" />
                          </button>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </div>

              {/* Info Banner */}
              <div className="bg-app-accent/10 border border-app-accent/20 rounded-lg p-4">
                <h4 className="text-sm font-bold text-app-accent mb-2">Important Notes</h4>
                <ul className="text-xs text-app-subtext space-y-1.5">
                  <li className="flex gap-2">
                    <div className="w-1 h-1 rounded-full bg-app-accent mt-1.5 shrink-0" />
                    Only tables in this allowlist can be queried through RAG
                  </li>
                  <li className="flex gap-2">
                    <div className="w-1 h-1 rounded-full bg-app-accent mt-1.5 shrink-0" />
                    Table names must match exactly (case-sensitive)
                  </li>
                  <li className="flex gap-2">
                    <div className="w-1 h-1 rounded-full bg-app-accent mt-1.5 shrink-0" />
                    Changes apply when creating or updating DB collections
                  </li>
                </ul>
              </div>
            </>
          )}
        </div>

        {/* Footer */}
        <div className="p-6 bg-app-card/30 border-t border-app-border flex justify-between items-center">
          <p className="text-xs text-app-subtext">
            {allowedTables.size} table(s) configured
          </p>
          <button
            onClick={onClose}
            className="px-5 py-2 bg-app-accent text-white rounded-lg text-sm font-bold shadow-lg shadow-app-accent/20 hover:bg-app-accent/90 transition-all"
          >
            Done
          </button>
        </div>
      </motion.div>
    </motion.div>
  );
}
