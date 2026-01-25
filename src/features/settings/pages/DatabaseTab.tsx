import { useState, useEffect } from "react";
import { useNavigate } from "react-router";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";
import {
  Plus,
  Info,
  Shield,
  Settings,
  Trash2,
  CheckCircle2,
  AlertCircle,
  Loader2,
} from "lucide-react";
import type {
  DbConnection,
  DbTestConnectionResult,
} from "../../../features/rag/types";

// Import extracted components
import { DbConnectionForm } from "../components/DbConnectionForm";
import { ConnectionTable } from "../components/ConnectionTable";
import { EmptyState } from "../components/EmptyState";
import { InfoCard } from "../components/InfoCard";

export default function DatabaseTab() {
  const [connections, setConnections] = useState<DbConnection[]>([]);
  const [showForm, setShowForm] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<{
    id: number;
    name: string;
  } | null>(null);
  const navigate = useNavigate();

  useEffect(() => {
    loadConnections();
  }, []);

  const loadConnections = async () => {
    setError(null);
    try {
      const result = await invoke<DbConnection[]>("db_list_connections");
      setConnections(result);
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : String(error);
      setError(`Failed to load connections: ${errorMsg}`);
      console.error("Failed to load connections:", error);
    } finally {
      setLoading(false);
    }
  };

  const testConnection = async (connId: number) => {
    setError(null);
    setSuccessMessage(null);
    try {
      const result = await invoke<DbTestConnectionResult>(
        "db_test_connection",
        { connId },
      );
      if (result.success) {
        setSuccessMessage(result.message);
        // Auto-hide success message after 5 seconds
        setTimeout(() => setSuccessMessage(null), 5000);
      } else {
        setError(result.message);
      }
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : String(error);
      setError(`Connection test failed: ${errorMsg}`);
    }
  };

  const deleteConnection = async (connId: number) => {
    setError(null);
    try {
      await invoke("db_delete_connection", { connId });
      setSuccessMessage("Connection deleted successfully");
      // Auto-hide success message after 3 seconds
      setTimeout(() => setSuccessMessage(null), 3000);
      loadConnections();
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : String(error);
      setError(`Failed to delete connection: ${errorMsg}`);
    } finally {
      setDeleteConfirm(null);
    }
  };

  const handleDeleteClick = (connId: number, connName: string) => {
    setDeleteConfirm({ id: connId, name: connName });
  };

  const cancelDelete = () => {
    setDeleteConfirm(null);
  };

  const confirmDelete = () => {
    if (deleteConfirm) {
      deleteConnection(deleteConfirm.id);
    }
  };

  // New handlers for profile and table management
  const handleConfigureProfile = (connection: DbConnection) => {
    navigate(`/database/setup/${connection.id}`);
  };

  const handleManageTables = (connection: DbConnection) => {
    navigate(`/database/setup/${connection.id}`);
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
          <h1 className="text-3xl font-bold tracking-tight text-app-text">
            Database Connections
          </h1>
          <p className="text-app-subtext max-w-2xl leading-relaxed">
            Configure matte-finished flat database connectors for your RAG
            pipeline. Optimized for performance and secure metadata querying.
          </p>
        </div>
        <button
          onClick={() => setShowForm(true)}
          className="bg-app-accent text-white px-5 py-2.5 rounded-lg flex items-center gap-2 text-sm font-bold shadow-lg shadow-app-accent/20 hover:bg-app-accent/90 transition-all hover:scale-[1.02] active:scale-[0.98]">
          <Plus className="w-4 h-4" />
          Add New Source
        </button>
      </div>

      {/* Error/Success Messages */}
      <AnimatePresence>
        {error && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            className="bg-destructive/10 text-destructive border border-destructive/20 rounded-lg p-4 flex items-start gap-3">
            <AlertCircle className="w-5 h-5 mt-0.5 shrink-0" />
            <div className="flex-1">
              <p className="text-sm font-medium">Error</p>
              <p className="text-sm mt-1">{error}</p>
            </div>
            <button
              onClick={() => setError(null)}
              className="p-1 hover:bg-destructive/20 rounded transition-colors">
              <Plus className="w-4 h-4 rotate-45" />
            </button>
          </motion.div>
        )}
        {successMessage && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            className="bg-app-success/10 text-app-success border border-app-success/20 rounded-lg p-4 flex items-start gap-3">
            <CheckCircle2 className="w-5 h-5 mt-0.5 shrink-0" />
            <div className="flex-1">
              <p className="text-sm font-medium">Success</p>
              <p className="text-sm mt-1">{successMessage}</p>
            </div>
            <button
              onClick={() => setSuccessMessage(null)}
              className="p-1 hover:bg-app-success/20 rounded transition-colors">
              <Plus className="w-4 h-4 rotate-45" />
            </button>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Info Banner Banner Architecture */}
      <div className="bg-app-card/30 border border-app-border rounded-xl p-5 flex gap-4">
        <div className="w-8 h-8 rounded-full bg-app-accent/10 flex items-center justify-center text-app-accent shrink-0">
          <Info className="w-4 h-4" />
        </div>
        <div className="space-y-3">
          <h3 className="text-sm font-bold text-app-text uppercase tracking-wider">
            Deployment Architecture
          </h3>
          <ul className="grid grid-cols-2 gap-x-12 gap-y-2">
            {[
              "Matte-finish connections are strictly isolated for external indexing.",
              "Whitelisted tables only. No direct root access enabled by default.",
              "SSL encryption is enforced for all flat-schemed database transactions.",
              "Optimized metadata resolution for high-concurrency vector lookups.",
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
            onTest={testConnection}
            onDeleteClick={handleDeleteClick}
            onConfigureProfile={handleConfigureProfile}
            onManageTables={handleManageTables}
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

      {/* Delete Confirmation Modal */}
      <AnimatePresence>
        {deleteConfirm && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4"
            onClick={cancelDelete}>
            <motion.div
              initial={{ scale: 0.95, opacity: 0, y: 20 }}
              animate={{ scale: 1, opacity: 1, y: 0 }}
              exit={{ scale: 0.95, opacity: 0, y: 20 }}
              className="bg-app-panel border border-app-border rounded-xl shadow-2xl w-full max-w-md overflow-hidden"
              onClick={(e) => e.stopPropagation()}>
              <div className="p-6">
                <div className="flex items-center gap-4 mb-4">
                  <div className="w-12 h-12 rounded-full bg-destructive/10 flex items-center justify-center text-destructive">
                    <Trash2 className="w-6 h-6" />
                  </div>
                  <div>
                    <h3 className="text-lg font-semibold text-app-text">
                      Delete Connection
                    </h3>
                    <p className="text-sm text-app-subtext">
                      This action cannot be undone
                    </p>
                  </div>
                </div>
                <p className="text-sm text-app-text mb-6">
                  Are you sure you want to delete{" "}
                  <span className="font-semibold">{deleteConfirm.name}</span>?
                  This will remove the connection configuration permanently.
                </p>
                <div className="flex justify-end gap-3">
                  <button
                    onClick={cancelDelete}
                    className="px-5 py-2 text-sm font-medium text-app-subtext hover:text-app-text transition-all">
                    Cancel
                  </button>
                  <button
                    onClick={confirmDelete}
                    className="px-5 py-2 bg-destructive text-white rounded-lg text-sm font-bold shadow-lg shadow-destructive/20 hover:bg-destructive/90 transition-all hover:scale-[1.02] active:scale-[0.98]">
                    Delete Connection
                  </button>
                </div>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
