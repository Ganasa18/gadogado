import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion } from "framer-motion";
import {
  Settings,
  Plus,
  Loader2,
  CheckCircle2,
} from "lucide-react";
import type { DbConnection, DbConnectionConfig } from "../../rag/types";
import { cn } from "../../../utils/cn";

interface DbConnectionConfigModalProps {
  connection: DbConnection | null;
  onClose: () => void;
  onSave: () => void;
}

export function DbConnectionConfigModal({
  connection,
  onClose,
  onSave,
}: DbConnectionConfigModalProps) {
  const [config, setConfig] = useState<DbConnectionConfig | null>(null);
  const [defaultLimit, setDefaultLimit] = useState<number>(50);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  useEffect(() => {
    if (connection) {
      loadConfig();
    }
  }, [connection]);

  const loadConfig = async () => {
    if (!connection) return;

    setLoading(true);
    setError(null);
    try {
      const parsed = await invoke<DbConnectionConfig>("db_get_connection_config", {
        connId: connection.id,
      });
      setConfig(parsed);
      setDefaultLimit(parsed.default_limit ?? 50);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : JSON.stringify(err);
      setError(`Failed to load config: ${errorMsg}`);
      // Set default values on error
      setConfig({
        profile_id: undefined,
        selected_tables: [],
        selected_columns: {},
        default_limit: 50,
        updated_at: undefined,
      });
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async () => {
    if (!connection || !config) return;

    setSaving(true);
    setError(null);
    setSuccess(false);

    try {
      const updatedConfig: DbConnectionConfig = {
        ...config,
        default_limit: defaultLimit,
        updated_at: new Date().toISOString(),
      };

      await invoke("db_save_connection_config", {
        connId: connection.id,
        configJson: JSON.stringify(updatedConfig),
      });

      setSuccess(true);
      setTimeout(() => {
        onSave();
      }, 500);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : JSON.stringify(err);
      setError(`Failed to save config: ${errorMsg}`);
    } finally {
      setSaving(false);
    }
  };

  if (!connection) return null;

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
              <Settings className="w-5 h-5" />
            </div>
            <div>
              <h2 className="text-lg font-semibold text-app-text">Connection Config</h2>
              <p className="text-xs text-app-subtext">{connection.name}</p>
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
          ) : error ? (
            <div className="bg-destructive/10 text-destructive border border-destructive/20 rounded-lg p-4 text-sm">
              {error}
            </div>
          ) : success ? (
            <div className="bg-app-success/10 text-app-success border border-app-success/20 rounded-lg p-4 flex items-center gap-3">
              <CheckCircle2 className="w-5 h-5" />
              <span className="text-sm font-medium">Configuration saved successfully!</span>
            </div>
          ) : config ? (
            <>
              {/* Default Limit */}
              <div>
                <label className="block text-xs font-bold uppercase tracking-wider text-app-subtext mb-2">
                  Default Query Limit
                </label>
                <div className="flex items-center gap-4">
                  <input
                    type="number"
                    min={1}
                    max={200}
                    value={defaultLimit}
                    onChange={(e) => {
                      const val = Math.min(200, Math.max(1, parseInt(e.target.value) || 50));
                      setDefaultLimit(val);
                    }}
                    className="w-32 bg-app-card border border-app-border rounded-lg px-4 py-2.5 text-app-text text-sm focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none transition-all"
                  />
                  <div className="flex-1">
                    <p className="text-xs text-app-subtext">
                      Maximum number of rows to return for "get all" queries (1-200).
                      This limit applies to all collections using this connection.
                    </p>
                  </div>
                </div>
              </div>

              {/* Current Configuration Info */}
              <div className="space-y-4">
                <h3 className="text-sm font-bold text-app-text uppercase tracking-wider">
                  Current Configuration
                </h3>

                <div className="bg-app-card/50 border border-app-border rounded-lg p-4 space-y-3">
                  <div className="flex justify-between items-center">
                    <span className="text-sm text-app-subtext">Profile ID</span>
                    <span className="text-sm font-mono text-app-text">
                      {config.profile_id ?? "Not set"}
                    </span>
                  </div>
                  <div className="flex justify-between items-center">
                    <span className="text-sm text-app-subtext">Selected Tables</span>
                    <span className="text-sm font-mono text-app-text">
                      {config.selected_tables.length} table(s)
                    </span>
                  </div>
                  <div>
                    <span className="text-sm text-app-subtext block mb-2">
                      Available Columns ({Object.keys(config.selected_columns).length} tables)
                    </span>
                    <div className="bg-app-bg border border-app-border rounded-lg p-3 max-h-32 overflow-y-auto">
                      {Object.keys(config.selected_columns).length > 0 ? (
                        <div className="flex flex-wrap gap-2">
                          {Object.entries(config.selected_columns).map(([table, cols]) => (
                            <div key={table} className="text-xs">
                              <span className="font-semibold text-app-accent">{table}</span>
                              <span className="text-app-subtext"> ({cols.length} cols)</span>
                            </div>
                          ))}
                        </div>
                      ) : (
                        <p className="text-xs text-app-subtext italic">No columns configured</p>
                      )}
                    </div>
                  </div>
                  {config.updated_at && (
                    <div className="flex justify-between items-center">
                      <span className="text-sm text-app-subtext">Last Updated</span>
                      <span className="text-xs text-app-subtext">
                        {new Date(config.updated_at).toLocaleString()}
                      </span>
                    </div>
                  )}
                </div>
              </div>

              {/* Info Box */}
              <div className="bg-app-accent/10 border border-app-accent/20 rounded-lg p-4">
                <p className="text-xs text-app-subtext leading-relaxed">
                  <strong className="text-app-accent">Note:</strong> This configuration
                  applies to all collections using this database connection. To change
                  which tables are available, use the "Setup Tables & Profile" option.
                </p>
              </div>
            </>
          ) : null}
        </div>

        {/* Footer */}
        <div className="p-6 bg-app-card/30 border-t border-app-border flex justify-between items-center">
          <button
            onClick={onClose}
            className="px-5 py-2 text-sm font-medium text-app-subtext hover:text-app-text transition-all"
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            disabled={saving || loading}
            className={cn(
              "px-6 py-2 rounded-lg text-sm font-bold shadow-lg transition-all flex items-center gap-2",
              saving || loading
                ? "bg-app-subtext/20 text-app-subtext cursor-not-allowed"
                : "bg-app-accent text-white hover:bg-app-accent/90 hover:scale-[1.02] active:scale-[0.98] shadow-app-accent/20"
            )}
          >
            {saving ? <Loader2 className="w-4 h-4 animate-spin" /> : "Save Config"}
          </button>
        </div>
      </motion.div>
    </motion.div>
  );
}
