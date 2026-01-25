import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";
import { X, Plus, Trash2, Loader2, CheckCircle2, AlertCircle } from "lucide-react";
import type { DbAllowlistProfile } from "../../rag/types";

interface ProfileRules {
  allowed_tables: Record<string, string[]>;
  require_filters: Record<string, string[]>;
  max_limit: number;
  allow_joins: boolean;
  deny_keywords: string[];
  deny_statements: string[];
}

interface ProfileManagementModalProps {
  isOpen: boolean;
  onClose: () => void;
  onProfileChange: () => void;
  currentProfiles: DbAllowlistProfile[];
}

const DEFAULT_RULES: ProfileRules = {
  allowed_tables: {},
  require_filters: {},
  max_limit: 200,
  allow_joins: false,
  deny_keywords: ["password", "token", "secret", "api_key"],
  deny_statements: ["INSERT", "UPDATE", "DELETE", "DROP", "ALTER", "PRAGMA", "ATTACH"]
};

export function ProfileManagementModal({
  isOpen,
  onClose,
  onProfileChange,
  currentProfiles
}: ProfileManagementModalProps) {
  const [mode, setMode] = useState<"list" | "create" | "edit">("list");
  const [editingProfile, setEditingProfile] = useState<DbAllowlistProfile | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  // Form state
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [maxLimit, setMaxLimit] = useState(200);
  const [allowJoins, setAllowJoins] = useState(false);
  const [denyKeywords, setDenyKeywords] = useState("password, token, secret, api_key");
  const [denyStatements, setDenyStatements] = useState("INSERT, UPDATE, DELETE, DROP, ALTER");

  const resetForm = () => {
    setName("");
    setDescription("");
    setMaxLimit(200);
    setAllowJoins(false);
    setDenyKeywords("password, token, secret, api_key");
    setDenyStatements("INSERT, UPDATE, DELETE, DROP, ALTER, PRAGMA, ATTACH");
  };

  const handleCreate = () => {
    setMode("create");
    resetForm();
    setError(null);
    setSuccess(null);
  };

  const handleEdit = (profile: DbAllowlistProfile) => {
    setMode("edit");
    setEditingProfile(profile);
    setError(null);
    setSuccess(null);

    // Parse existing rules
    const rules: ProfileRules = JSON.parse(profile.rules_json);
    setName(profile.name);
    setDescription(profile.description || "");
    setMaxLimit(rules.max_limit || 200);
    setAllowJoins(rules.allow_joins || false);
    setDenyKeywords(rules.deny_keywords?.join(", ") || "password, token, secret, api_key");
    setDenyStatements(rules.deny_statements?.join(", ") || "INSERT, UPDATE, DELETE, DROP, ALTER");
  };

  const handleDelete = async (profileId: number, profileName: string) => {
    if (profileId === 1) {
      setError("Cannot delete default profile");
      return;
    }

    if (!confirm(`Are you sure you want to delete "${profileName}"?`)) {
      return;
    }

    setLoading(true);
    setError(null);
    try {
      await invoke("db_delete_allowlist_profile", { profileId });
      setSuccess(`Profile "${profileName}" deleted successfully`);
      onProfileChange();
      setTimeout(() => setSuccess(null), 2000);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(`Failed to delete: ${msg}`);
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async () => {
    if (!name.trim()) {
      setError("Profile name is required");
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const rules: ProfileRules = {
        allowed_tables: {},
        require_filters: {},
        max_limit: maxLimit,
        allow_joins: allowJoins,
        deny_keywords: denyKeywords.split(",").map(k => k.trim()).filter(k => k),
        deny_statements: denyStatements.split(",").map(s => s.trim()).filter(s => s)
      };

      if (mode === "create") {
        await invoke("db_create_allowlist_profile", {
          name: name.trim(),
          description: description.trim() || null,
          rulesJson: JSON.stringify(rules)
        });
        setSuccess(`Profile "${name}" created successfully`);
      } else {
        await invoke("db_update_allowlist_profile", {
          profileId: editingProfile!.id,
          name: name.trim(),
          description: description.trim() || null,
          rulesJson: JSON.stringify(rules)
        });
        setSuccess(`Profile "${name}" updated successfully`);
      }

      onProfileChange();
      setTimeout(() => {
        setSuccess(null);
        setMode("list");
        resetForm();
        setEditingProfile(null);
      }, 1500);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(`Failed to save: ${msg}`);
    } finally {
      setLoading(false);
    }
  };

  if (!isOpen) return null;

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
        className="bg-app-panel border border-app-border rounded-xl shadow-2xl w-full max-w-2xl overflow-hidden max-h-[80vh] flex flex-col"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="px-6 py-4 border-b border-app-border flex justify-between items-center bg-app-card/30">
          <h2 className="text-lg font-semibold text-app-text">
            {mode === "list" ? "Manage Profiles" : mode === "create" ? "Create New Profile" : "Edit Profile"}
          </h2>
          <button onClick={onClose} className="p-2 hover:bg-app-card rounded-full text-app-subtext">
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 overflow-y-auto flex-1">
          {/* Messages */}
          <AnimatePresence>
            {error && (
              <motion.div
                initial={{ opacity: 0, height: 0 }}
                animate={{ opacity: 1, height: "auto" }}
                exit={{ opacity: 0, height: 0 }}
                className="mb-4 bg-destructive/10 text-destructive border border-destructive/20 rounded-lg p-3 text-sm flex items-start gap-2"
              >
                <AlertCircle className="w-4 h-4 mt-0.5 shrink-0" />
                <span>{error}</span>
              </motion.div>
            )}
            {success && (
              <motion.div
                initial={{ opacity: 0, height: 0 }}
                animate={{ opacity: 1, height: "auto" }}
                exit={{ opacity: 0, height: 0 }}
                className="mb-4 bg-app-success/10 text-app-success border border-app-success/20 rounded-lg p-3 text-sm flex items-start gap-2"
              >
                <CheckCircle2 className="w-4 h-4 mt-0.5 shrink-0" />
                <span>{success}</span>
              </motion.div>
            )}
          </AnimatePresence>

          {mode === "list" ? (
            /* Profile List */
            <div className="space-y-3">
              <div className="flex justify-between items-center">
                <p className="text-sm text-app-subtext">Manage security profiles for database access</p>
                <button
                  onClick={handleCreate}
                  className="px-3 py-1.5 bg-app-accent text-white rounded-lg text-sm font-bold hover:bg-app-accent/90 transition-all flex items-center gap-1"
                >
                  <Plus className="w-4 h-4" />
                  Create New
                </button>
              </div>

              {currentProfiles.map((profile) => (
                <div
                  key={profile.id}
                  className="p-4 bg-app-card/40 border border-app-border rounded-xl flex justify-between items-start"
                >
                  <div className="flex-1">
                    <div className="flex items-center gap-2">
                      <h3 className="font-semibold text-app-text">{profile.name}</h3>
                      {profile.id === 1 && (
                        <span className="px-2 py-0.5 text-xs bg-app-accent/20 text-app-accent rounded-full font-medium">
                          Default
                        </span>
                      )}
                    </div>
                    <p className="text-sm text-app-subtext mt-1">{profile.description || "No description"}</p>
                  </div>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => handleEdit(profile)}
                      className="px-3 py-1.5 text-sm font-medium text-app-accent hover:bg-app-accent/10 rounded-lg transition-colors"
                    >
                      Edit
                    </button>
                    {profile.id !== 1 && (
                      <button
                        onClick={() => handleDelete(profile.id, profile.name)}
                        disabled={loading}
                        className="p-1.5 text-app-subtext hover:text-destructive hover:bg-destructive/10 rounded-lg transition-colors disabled:opacity-50"
                      >
                        <Trash2 className="w-4 h-4" />
                      </button>
                    )}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            /* Create/Edit Form */
            <div className="space-y-4">
              <button
                onClick={() => setMode("list")}
                className="text-sm text-app-accent hover:text-app-accent/80"
              >
                ← Back to list
              </button>

              <div>
                <label className="block text-sm font-medium text-app-text mb-1">Profile Name *</label>
                <input
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="e.g., Strict Access, Developer Profile"
                  className="w-full bg-app-card border border-app-border rounded-lg px-4 py-2 text-sm focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-app-text mb-1">Description</label>
                <textarea
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  placeholder="Describe this profile's purpose..."
                  rows={2}
                  className="w-full bg-app-card border border-app-border rounded-lg px-4 py-2 text-sm focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none resize-none"
                />
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-app-text mb-1">Max Limit</label>
                  <input
                    type="number"
                    value={maxLimit}
                    onChange={(e) => setMaxLimit(parseInt(e.target.value) || 200)}
                    className="w-full bg-app-card border border-app-border rounded-lg px-4 py-2 text-sm focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none"
                  />
                </div>
                <div className="flex items-center gap-2 pt-6">
                  <input
                    type="checkbox"
                    id="allowJoins"
                    checked={allowJoins}
                    onChange={(e) => setAllowJoins(e.target.checked)}
                    className="w-4 h-4 rounded border-app-border"
                  />
                  <label htmlFor="allowJoins" className="text-sm text-app-text">Allow JOIN Queries</label>
                </div>
              </div>

              <div>
                <label className="block text-sm font-medium text-app-text mb-1">Denied Keywords (comma-separated)</label>
                <input
                  type="text"
                  value={denyKeywords}
                  onChange={(e) => setDenyKeywords(e.target.value)}
                  className="w-full bg-app-card border border-app-border rounded-lg px-4 py-2 text-sm focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none font-mono text-xs"
                />
                <p className="text-xs text-app-subtext mt-1">Queries containing these keywords will be blocked</p>
              </div>

              <div>
                <label className="block text-sm font-medium text-app-text mb-1">Denied SQL Statements (comma-separated)</label>
                <input
                  type="text"
                  value={denyStatements}
                  onChange={(e) => setDenyStatements(e.target.value)}
                  className="w-full bg-app-card border border-app-border rounded-lg px-4 py-2 text-sm focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none font-mono text-xs"
                />
                <p className="text-xs text-app-subtext mt-1">These SQL statement types will be blocked</p>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        {mode !== "list" && (
          <div className="px-6 py-4 bg-app-card/30 border-t border-app-border flex justify-end gap-3">
            <button
              onClick={() => setMode("list")}
              className="px-4 py-2 text-sm font-medium text-app-subtext hover:text-app-text"
              disabled={loading}
            >
              Cancel
            </button>
            <button
              onClick={handleSave}
              disabled={loading}
              className="px-5 py-2 bg-app-accent text-white rounded-lg text-sm font-bold shadow-lg shadow-app-accent/20 hover:bg-app-accent/90 transition-all disabled:opacity-50 flex items-center gap-2"
            >
              {loading ? <Loader2 className="w-4 h-4 animate-spin" /> : null}
              {loading ? "Saving..." : "Save Profile"}
            </button>
          </div>
        )}
      </motion.div>
    </motion.div>
  );
}
