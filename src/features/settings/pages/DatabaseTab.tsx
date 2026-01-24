import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";
import { 
  Database, 
  Plus, 
  Info, 
  Shield, 
  Settings, 
  MoreVertical, 
  Trash2, 
  CheckCircle2, 
  AlertCircle, 
  Loader2,
  ChevronRight,
  Eye,
  EyeOff,
  Lock,
  RefreshCw
} from "lucide-react";
import type {
  DbConnection,
  DbConnectionInput,
  DbTestConnectionResult,
} from "../../../features/rag/types";
import { cn } from "../../../utils/cn";

interface DbConnectionFormProps {
  onClose: () => void;
  onSave: () => void;
}

function DbConnectionForm({ onClose, onSave }: DbConnectionFormProps) {
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

const ConnectionTable = ({ 
  connections, 
  onDelete, 
  onTest 
}: { 
  connections: DbConnection[], 
  onDelete: (id: number) => void,
  onTest: (id: number) => void
}) => {
  return (
    <div className="bg-app-panel border border-app-border rounded-xl overflow-hidden shadow-xl">
      <table className="w-full text-left border-collapse">
        <thead>
          <tr className="bg-app-card/50 text-app-subtext border-b border-app-border">
            <th className="px-6 py-4 text-[10px] font-bold uppercase tracking-widest">Name</th>
            <th className="px-6 py-4 text-[10px] font-bold uppercase tracking-widest">Status</th>
            <th className="px-6 py-4 text-[10px] font-bold uppercase tracking-widest">Type</th>
            <th className="px-6 py-4 text-[10px] font-bold uppercase tracking-widest text-right">Actions</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-app-border/40">
          {connections.map((conn) => (
            <tr key={conn.id} className="hover:bg-app-card/30 transition-colors group">
              <td className="px-6 py-5">
                <div className="flex items-center gap-3">
                  <div className="w-10 h-10 rounded-lg bg-app-card flex items-center justify-center text-app-subtext group-hover:text-app-accent transition-colors">
                    <Database className="w-5 h-5" />
                  </div>
                  <div>
                    <div className="text-sm font-semibold text-app-text">{conn.name}</div>
                    <div className="text-xs text-app-subtext mt-0.5">{conn.host}:{conn.port} • {conn.database_name}</div>
                  </div>
                </div>
              </td>
              <td className="px-6 py-5">
                <span className={cn(
                  "px-2.5 py-1 rounded-full text-[10px] font-bold uppercase tracking-wider inline-flex items-center gap-1.5",
                  conn.is_enabled 
                    ? "bg-app-success/10 text-app-success border border-app-success/20" 
                    : "bg-app-subtext/10 text-app-subtext border border-app-subtext/20"
                )}>
                  <div className={cn("w-1.5 h-1.5 rounded-full", conn.is_enabled ? "bg-app-success" : "bg-app-subtext")} />
                  {conn.is_enabled ? "Active" : "Disabled"}
                </span>
              </td>
              <td className="px-6 py-5">
                <div className="text-sm text-app-text capitalize">{conn.db_type}</div>
              </td>
              <td className="px-6 py-5">
                <div className="flex justify-end gap-2">
                  <button 
                    onClick={() => onTest(conn.id)}
                    className="p-2 text-app-subtext hover:text-app-text hover:bg-app-card rounded-lg transition-all"
                    title="Test Connection"
                  >
                    <RefreshCw className="w-4 h-4" />
                  </button>
                  <button 
                    onClick={() => onDelete(conn.id)}
                    className="p-2 text-app-subtext hover:text-destructive hover:bg-destructive/10 rounded-lg transition-all"
                    title="Delete Connection"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                  <button className="p-2 text-app-subtext hover:text-app-text hover:bg-app-card rounded-lg transition-all">
                    <MoreVertical className="w-4 h-4" />
                  </button>
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};

const EmptyState = ({ onInitialize }: { onInitialize: () => void }) => (
  <div className="bg-app-panel border border-app-border rounded-xl p-12 flex flex-col items-center justify-center text-center shadow-xl">
    <div className="w-16 h-16 rounded-2xl bg-app-card flex items-center justify-center text-app-subtext mb-6">
      <Database className="w-8 h-8 opacity-40" />
    </div>
    <h3 className="text-xl font-bold text-app-text mb-2">No connectors established</h3>
    <p className="text-app-subtext max-w-sm mb-8">
      Start by defining your first flat-schemed database source to enable hybrid RAG operations.
    </p>
    <button 
      onClick={onInitialize}
      className="flex items-center gap-2 text-app-accent font-bold text-sm group"
    >
      Initialize Connection
      <ChevronRight className="w-4 h-4 group-hover:translate-x-1 transition-transform" />
    </button>
  </div>
);

const InfoCard = ({ icon: Icon, title, description }: { icon: any, title: string, description: string }) => (
  <div className="bg-app-panel border border-app-border rounded-xl p-5 flex gap-4 shadow-md">
    <div className="w-10 h-10 rounded-lg bg-app-card shrink-0 flex items-center justify-center text-app-subtext">
      <Icon className="w-5 h-5" />
    </div>
    <div>
      <h4 className="text-sm font-bold text-app-text mb-1 uppercase tracking-tight">{title}</h4>
      <p className="text-xs text-app-subtext leading-relaxed">{description}</p>
    </div>
  </div>
);

export default function DatabaseTab() {
  const [connections, setConnections] = useState<DbConnection[]>([]);
  const [showForm, setShowForm] = useState(false);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadConnections();
  }, []);

  const loadConnections = async () => {
    try {
      const result = await invoke<DbConnection[]>("db_list_connections");
      setConnections(result);
    } catch (error) {
      console.error("Failed to load connections:", error);
    } finally {
      setLoading(false);
    }
  };

  const testOldConnection = async (connId: number) => {
    try {
      const result = await invoke<DbTestConnectionResult>("db_test_connection", { connId });
      alert(result.message);
    } catch (error) {
      alert("Connection test failed: " + error);
    }
  };

  const deleteConnection = async (connId: number) => {
    if (!confirm("Are you sure you want to delete this connection?")) {
      return;
    }

    try {
      await invoke("db_delete_connection", { connId });
      loadConnections();
    } catch (error) {
      alert("Failed to delete connection: " + error);
    }
  };

  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <Loader2 className="w-8 h-8 animate-spin text-app-accent" />
      </div>
    );
  }

  return (
    <div className="p-8 space-y-8 max-w-6xl mx-auto">
      {/* Header section matching designs */}
      <div className="flex justify-between items-start">
        <div className="space-y-1">
          <h1 className="text-3xl font-bold tracking-tight text-app-text">Database Connections</h1>
          <p className="text-app-subtext max-w-2xl leading-relaxed">
            Configure matte-finished flat database connectors for your RAG pipeline. Optimized for performance and secure metadata querying.
          </p>
        </div>
        <button
          onClick={() => setShowForm(true)}
          className="bg-app-accent text-white px-5 py-2.5 rounded-lg flex items-center gap-2 text-sm font-bold shadow-lg shadow-app-accent/20 hover:bg-app-accent/90 transition-all hover:scale-[1.02] active:scale-[0.98]"
        >
          <Plus className="w-4 h-4" />
          Add New Source
        </button>
      </div>

      {/* Info Banner Banner Architecture */}
      <div className="bg-app-card/30 border border-app-border rounded-xl p-5 flex gap-4">
        <div className="w-8 h-8 rounded-full bg-app-accent/10 flex items-center justify-center text-app-accent shrink-0">
          <Info className="w-4 h-4" />
        </div>
        <div className="space-y-3">
          <h3 className="text-sm font-bold text-app-text uppercase tracking-wider">Deployment Architecture</h3>
          <ul className="grid grid-cols-2 gap-x-12 gap-y-2">
            {[
              "Matte-finish connections are strictly isolated for external indexing.",
              "Whitelisted tables only. No direct root access enabled by default.",
              "SSL encryption is enforced for all flat-schemed database transactions.",
              "Optimized metadata resolution for high-concurrency vector lookups."
            ].map((text, i) => (
              <li key={i} className="text-xs text-app-subtext flex gap-2">
                <div className="w-1.5 h-1.5 rounded-full bg-app-accent shrink-0 mt-1" />
                {text}
              </li>
            ))}
          </ul>
        </div>
      </div>

      {/* Main Content: Table or Empty State */}
      <div className="min-h-[300px]">
        {connections.length === 0 ? (
          <EmptyState onInitialize={() => setShowForm(true)} />
        ) : (
          <ConnectionTable 
            connections={connections} 
            onDelete={deleteConnection}
            onTest={testOldConnection}
          />
        )}
      </div>

      {/* Policy and Config cards at bottom */}
      <div className="grid grid-cols-2 gap-6 pt-4">
        <InfoCard 
          icon={Shield}
          title="Security Policy"
          description="SSL encryption is strictly enforced. All database credentials are encrypted at rest using AES-256 flat-key rotation."
        />
        <InfoCard 
          icon={Settings}
          title="RAG Configuration"
          description="Indexing frequency set to 15-minute intervals for active connectors. Vector embeddings are cached for 24 hours."
        />
      </div>

      {/* Modal Form */}
      <AnimatePresence>
        {showForm && (
          <DbConnectionForm
            onClose={() => setShowForm(false)}
            onSave={() => {
              setShowForm(false);
              loadConnections();
            }}
          />
        )}
      </AnimatePresence>
    </div>
  );
}
