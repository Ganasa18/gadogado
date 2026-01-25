import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";
import {
  Database,
  Plus,
  CheckCircle2,
  AlertCircle,
  Loader2,
  Eye,
  EyeOff,
  Lock,
  RefreshCw,
  ChevronRight,
} from "lucide-react";
import type {
  DbConnection,
  DbConnectionInput,
  DbTestConnectionResult,
} from "../../rag/types";
import { cn } from "../../../utils/cn";

interface DbConnectionFormProps {
  onClose: () => void;
  onSave: () => void;
}

export function DbConnectionForm({ onClose, onSave }: DbConnectionFormProps) {
  const [formData, setFormData] = useState<DbConnectionInput>({
    name: "",
    db_type: "postgres",
    host: "localhost",
    port: 5432,
    database_name: "",
    username: "",
    password: "",
    ssl_mode: "require",
  });
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);
  const [showPassword, setShowPassword] = useState(false);
  const [testResult, setTestResult] = useState<DbTestConnectionResult | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setTestResult(null);

    try {
      await invoke<DbConnection>("db_add_connection", { input: formData });
      onSave();
    } catch (error) {
      setTestResult({ success: false, message: String(error) });
    } finally {
      setSaving(false);
    }
  };

  const handleTest = async () => {
    setTesting(true);
    setTestResult(null);
    try {
      const result = await invoke<DbTestConnectionResult>("db_test_connection_input", { input: formData });
      setTestResult(result);
    } catch (error) {
      setTestResult({ success: false, message: "Connection test failed: " + error });
    } finally {
      setTesting(false);
    }
  };

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4"
    >
      <motion.div
        initial={{ scale: 0.95, opacity: 0, y: 20 }}
        animate={{ scale: 1, opacity: 1, y: 0 }}
        exit={{ scale: 0.95, opacity: 0, y: 20 }}
        className="bg-app-panel border border-app-border rounded-xl shadow-2xl w-full max-w-2xl overflow-hidden flex flex-col"
      >
        <div className="px-6 py-4 border-b border-app-border flex justify-between items-center bg-app-card/30">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-lg bg-app-accent/10 flex items-center justify-center text-app-accent">
              <Database className="w-5 h-5" />
            </div>
            <div>
              <h2 className="text-lg font-semibold text-app-text">Add New Database</h2>
              <p className="text-xs text-app-subtext">Configure a new flat database source for RAG.</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-2 hover:bg-app-card rounded-full text-app-subtext transition-colors"
          >
            <Plus className="w-5 h-5 rotate-45" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="p-6 space-y-6 overflow-y-auto max-h-[70vh]">
          <div className="grid grid-cols-2 gap-4">
            {/* Connection Name */}
            <div className="col-span-1">
              <label className="block text-xs font-bold uppercase tracking-wider text-app-subtext mb-1.5 ml-1">
                Connection Name
              </label>
              <input
                type="text"
                required
                value={formData.name}
                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                placeholder="Production PostgreSQL"
                className="w-full bg-app-card border border-app-border rounded-lg px-4 py-2.5 text-app-text text-sm focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none transition-all"
              />
            </div>

            {/* Database Type */}
            <div className="col-span-1">
              <label className="block text-xs font-bold uppercase tracking-wider text-app-subtext mb-1.5 ml-1">
                Database Type
              </label>
              <div className="relative">
                <select
                  value={formData.db_type}
                  onChange={(e) => setFormData({ ...formData, db_type: e.target.value as "postgres" | "sqlite" })}
                  className="w-full appearance-none bg-app-card border border-app-border rounded-lg px-4 py-2.5 text-app-text text-sm focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none transition-all cursor-pointer"
                >
                  <option value="postgres">PostgreSQL</option>
                  <option value="sqlite">SQLite</option>
                </select>
                <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-app-subtext">
                  <RefreshCw className="w-3.5 h-3.5" />
                </div>
              </div>
            </div>
          </div>

          <div className="grid grid-cols-3 gap-4">
            {/* Host */}
            <div className="col-span-2">
              <label className="block text-xs font-bold uppercase tracking-wider text-app-subtext mb-1.5 ml-1">
                Host
              </label>
              <input
                type="text"
                required
                value={formData.host}
                onChange={(e) => setFormData({ ...formData, host: e.target.value })}
                placeholder="db.example.com"
                className="w-full bg-app-card border border-app-border rounded-lg px-4 py-2.5 text-app-text text-sm focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none transition-all"
              />
            </div>

            {/* Port */}
            <div className="col-span-1">
              <label className="block text-xs font-bold uppercase tracking-wider text-app-subtext mb-1.5 ml-1">
                Port
              </label>
              <input
                type="number"
                required
                value={formData.port}
                onChange={(e) => setFormData({ ...formData, port: parseInt(e.target.value) || 0 })}
                placeholder="5432"
                className="w-full bg-app-card border border-app-border rounded-lg px-4 py-2.5 text-app-text text-sm focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none transition-all"
              />
            </div>
          </div>

          {/* Database Name */}
          <div>
            <label className="block text-xs font-bold uppercase tracking-wider text-app-subtext mb-1.5 ml-1">
              Database Name
            </label>
            <input
              type="text"
              required
              value={formData.database_name}
              onChange={(e) => setFormData({ ...formData, database_name: e.target.value })}
              placeholder="main_db"
              className="w-full bg-app-card border border-app-border rounded-lg px-4 py-2.5 text-app-text text-sm focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none transition-all"
            />
          </div>

          <div className="grid grid-cols-2 gap-4">
            {/* Username */}
            <div className="col-span-1">
              <label className="block text-xs font-bold uppercase tracking-wider text-app-subtext mb-1.5 ml-1">
                Username
              </label>
              <input
                type="text"
                required
                value={formData.username}
                onChange={(e) => setFormData({ ...formData, username: e.target.value })}
                placeholder="postgres"
                className="w-full bg-app-card border border-app-border rounded-lg px-4 py-2.5 text-app-text text-sm focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none transition-all"
              />
            </div>

            {/* Password */}
            <div className="col-span-1">
              <label className="block text-xs font-bold uppercase tracking-wider text-app-subtext mb-1.5 ml-1">
                Password
              </label>
              <div className="relative">
                <input
                  type={showPassword ? "text" : "password"}
                  required
                  value={formData.password}
                  onChange={(e) => setFormData({ ...formData, password: e.target.value })}
                  placeholder="••••••••••••"
                  className="w-full bg-app-card border border-app-border rounded-lg px-4 py-2.5 text-app-text text-sm focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none transition-all pr-10"
                />
                <button
                  type="button"
                  onClick={() => setShowPassword(!showPassword)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-app-subtext hover:text-app-text transition-colors"
                >
                  {showPassword ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                </button>
              </div>
              <div className="flex items-center gap-1.5 mt-2 text-[10px] text-app-subtext uppercase font-bold tracking-widest pl-1">
                <Lock className="w-2.5 h-2.5" />
                Stored in secure keychain
              </div>
            </div>
          </div>

          {/* SSL Mode */}
          <div>
            <label className="block text-xs font-bold uppercase tracking-wider text-app-subtext mb-1.5 ml-1">
              SSL Mode
            </label>
            <div className="relative">
              <select
                value={formData.ssl_mode}
                onChange={(e) => setFormData({ ...formData, ssl_mode: e.target.value })}
                className="w-full appearance-none bg-app-card border border-app-border rounded-lg px-4 py-2.5 text-app-text text-sm focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none transition-all cursor-pointer"
              >
                <option value="require">Require (Recommended)</option>
                <option value="prefer">Prefer</option>
                <option value="disable">Disable (Not Recommended)</option>
              </select>
              <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-app-subtext">
                <ChevronRight className="w-3.5 h-3.5 rotate-90" />
              </div>
            </div>
          </div>

          {/* Test Result Message */}
          <AnimatePresence>
            {testResult && (
              <motion.div
                initial={{ opacity: 0, height: 0 }}
                animate={{ opacity: 1, height: 'auto' }}
                exit={{ opacity: 0, height: 0 }}
                className={cn(
                  "p-3 rounded-lg text-sm flex items-start gap-3",
                  testResult.success
                    ? "bg-app-success/10 text-app-success border border-app-success/20"
                    : "bg-destructive/10 text-destructive border border-destructive/20"
                )}
              >
                {testResult.success ? <CheckCircle2 className="w-4 h-4 mt-0.5 shrink-0" /> : <AlertCircle className="w-4 h-4 mt-0.5 shrink-0" />}
                <span>{testResult.message}</span>
              </motion.div>
            )}
          </AnimatePresence>
        </form>

        <div className="p-6 bg-app-card/30 border-t border-app-border flex justify-between items-center">
          <button
            type="button"
            onClick={handleTest}
            disabled={testing || saving}
            className="px-5 py-2 text-sm font-medium text-app-text hover:bg-app-card rounded-lg transition-all flex items-center gap-2 border border-app-border disabled:opacity-50"
          >
            {testing ? <Loader2 className="w-4 h-4 animate-spin text-app-accent" /> : "Test Connection"}
          </button>

          <div className="flex gap-3">
            <button
              type="button"
              onClick={onClose}
              className="px-5 py-2 text-sm font-medium text-app-subtext hover:text-app-text transition-all"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={saving || testing}
              onClick={handleSubmit}
              className="px-6 py-2 bg-app-accent text-white rounded-lg text-sm font-bold shadow-lg shadow-app-accent/20 hover:bg-app-accent/90 hover:scale-[1.02] active:scale-[0.98] transition-all disabled:opacity-50 flex items-center gap-2"
            >
              {saving ? <Loader2 className="w-4 h-4 animate-spin" /> : "Save Connection"}
            </button>
          </div>
        </div>
      </motion.div>
    </motion.div>
  );
}
