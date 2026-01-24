import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { 
  FileText, 
  Database, 
  Info, 
  X,
  ChevronRight,
  Shield,
  Zap,
  Globe
} from "lucide-react";
import { cn } from "../../../utils/cn";

export default function SetupTab() {
  const [showInfo, setShowInfo] = useState(false);
  const [selectedType, setSelectedType] = useState<"files" | "database" | null>(null);

  return (
    <div className="flex-1 bg-app-bg text-app-text min-h-screen p-8 relative overflow-hidden">
      {/* Background Decorative Elements */}
      <div className="absolute top-[-10%] right-[-10%] w-[400px] h-[400px] bg-app-accent/5 rounded-full blur-[100px] pointer-events-none" />
      <div className="absolute bottom-[-10%] left-[-10%] w-[300px] h-[300px] bg-blue-500/5 rounded-full blur-[80px] pointer-events-none" />

      <div className="max-w-4xl mx-auto h-full flex flex-col items-center justify-center space-y-12">
        {/* Header Section */}
        <div className="text-center space-y-4 relative w-full">
          {/* Info Icon Button - Positioned top right of the container */}
          <div className="absolute top-0 right-0">
            <button
              onClick={() => setShowInfo(true)}
              className="p-2.5 rounded-full bg-blue-500/10 border border-blue-500/30 text-blue-400 hover:bg-blue-500/20 hover:scale-110 transition-all shadow-lg shadow-blue-500/10 active:scale-95"
              title="Click for more information"
            >
              <Info className="w-5 h-5 shadow-sm" />
            </button>
          </div>

          <motion.div
            initial={{ opacity: 0, y: -20 }}
            animate={{ opacity: 1, y: 0 }}
            className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-app-accent/10 border border-app-accent/20 text-app-accent text-[10px] font-bold uppercase tracking-widest"
          >
            <Zap className="w-3 h-3" />
            Pipeline Initialization
          </motion.div>
          
          <motion.h1 
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.1 }}
            className="text-4xl font-bold tracking-tight text-white mb-2"
          >
            Setup Connection
          </motion.h1>
          <motion.p 
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 0.2 }}
            className="text-app-subtext text-lg max-w-md mx-auto leading-relaxed"
          >
            Select your data architecture to begin the automated model distillation process.
          </motion.p>
        </div>

        {/* Selection Cards */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-8 w-full max-w-3xl">
          {/* Files Card */}
          <motion.div
            initial={{ opacity: 0, x: -30 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: 0.3 }}
            onClick={() => setSelectedType("files")}
            className={cn(
              "group relative p-8 rounded-2xl border-2 transition-all duration-300 cursor-pointer overflow-hidden",
              selectedType === "files" 
                ? "bg-app-accent/10 border-app-accent shadow-2xl shadow-app-accent/10" 
                : "bg-app-panel border-app-border hover:border-app-subtext/40 hover:bg-app-card/40"
            )}
          >
            <div className="relative z-10 flex flex-col items-center text-center space-y-6">
              <div className={cn(
                "w-20 h-20 rounded-2xl flex items-center justify-center transition-all duration-500",
                selectedType === "files" ? "bg-app-accent text-white scale-110" : "bg-app-card text-app-subtext group-hover:scale-110 group-hover:text-white"
              )}>
                <FileText className="w-10 h-10" />
              </div>
              <div className="space-y-2">
                <h3 className="text-2xl font-bold text-app-text group-hover:text-white transition-colors">Files</h3>
                <p className="text-sm text-app-subtext leading-relaxed">
                  Import local documents, datasets, or structured JSON/CSV files for direct processing.
                </p>
              </div>
              <div className={cn(
                "flex items-center gap-1.5 text-xs font-bold uppercase tracking-widest transition-all",
                selectedType === "files" ? "text-app-accent opacity-100" : "text-app-subtext opacity-0 group-hover:opacity-100"
              )}>
                Initialize Files <ChevronRight className="w-3 h-3" />
              </div>
            </div>
            
            {/* Hover Gradient Effect */}
            <div className="absolute inset-0 bg-gradient-to-br from-app-accent/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none" />
          </motion.div>

          {/* Database Card */}
          <motion.div
            initial={{ opacity: 0, x: 30 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: 0.4 }}
            onClick={() => setSelectedType("database")}
            className={cn(
              "group relative p-8 rounded-2xl border-2 transition-all duration-300 cursor-pointer overflow-hidden",
              selectedType === "database" 
                ? "bg-blue-500/10 border-blue-500 shadow-2xl shadow-blue-500/10" 
                : "bg-app-panel border-app-border hover:border-app-subtext/40 hover:bg-app-card/40"
            )}
          >
            <div className="relative z-10 flex flex-col items-center text-center space-y-6">
              <div className={cn(
                "w-20 h-20 rounded-2xl flex items-center justify-center transition-all duration-500",
                selectedType === "database" ? "bg-blue-500 text-white scale-110" : "bg-app-card text-app-subtext group-hover:scale-110 group-hover:text-white"
              )}>
                <Database className="w-10 h-10" />
              </div>
              <div className="space-y-2">
                <h3 className="text-2xl font-bold text-app-text group-hover:text-white transition-colors">Database</h3>
                <p className="text-sm text-app-subtext leading-relaxed">
                  Connect to SQL clusters or external vector stores for large-scale knowledge retrieval.
                </p>
              </div>
              <div className={cn(
                "flex items-center gap-1.5 text-xs font-bold uppercase tracking-widest transition-all",
                selectedType === "database" ? "text-blue-400 opacity-100" : "text-app-subtext opacity-0 group-hover:opacity-100"
              )}>
                Establish Link <ChevronRight className="w-3 h-3" />
              </div>
            </div>

            {/* Hover Gradient Effect */}
            <div className="absolute inset-0 bg-gradient-to-br from-blue-500/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none" />
          </motion.div>
        </div>

        {/* Action Button - Only visible if selected */}
        <AnimatePresence>
          {selectedType && (
            <motion.div
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: 20 }}
              className="w-full max-w-xs"
            >
              <button className="w-full py-4 bg-white text-black font-black uppercase tracking-widest rounded-xl shadow-2xl hover:bg-gray-100 hover:scale-[1.02] active:scale-[0.98] transition-all flex items-center justify-center gap-3">
                Continue Setup
                <ChevronRight className="w-5 h-5" />
              </button>
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      {/* Info Popup Modal */}
      <AnimatePresence>
        {showInfo && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-50 flex items-center justify-center p-6 bg-black/60 backdrop-blur-md"
          >
            <motion.div
              initial={{ scale: 0.9, opacity: 0, y: 20 }}
              animate={{ scale: 1, opacity: 1, y: 0 }}
              exit={{ scale: 0.9, opacity: 0, y: 20 }}
              className="bg-app-panel border border-app-border rounded-3xl w-full max-w-lg shadow-2xl overflow-hidden relative"
            >
              {/* Modal Header */}
              <div className="p-6 border-b border-app-border flex justify-between items-center bg-app-card/30">
                <div className="flex items-center gap-3">
                  <div className="p-2 rounded-lg bg-blue-500/10 text-blue-400">
                    <Info className="w-5 h-5" />
                  </div>
                  <h2 className="text-xl font-bold text-white uppercase tracking-tight">Configuration Details</h2>
                </div>
                <button
                  onClick={() => setShowInfo(false)}
                  className="p-2 hover:bg-app-card rounded-full text-app-subtext hover:text-white transition-colors"
                >
                  <X className="w-5 h-5" />
                </button>
              </div>

              {/* Modal Body */}
              <div className="p-8 space-y-6">
                <div className="space-y-4">
                  <div className="flex gap-4">
                    <div className="w-10 h-10 rounded-xl bg-app-card shrink-0 flex items-center justify-center text-app-accent">
                      <Shield className="w-5 h-5" />
                    </div>
                    <div>
                      <h4 className="text-sm font-bold text-white uppercase tracking-wider mb-1">Secure Isolation</h4>
                      <p className="text-sm text-app-subtext leading-relaxed">
                        The distillation pipeline runs in a sandboxed environment. Your data sources remain read-only during extraction.
                      </p>
                    </div>
                  </div>

                  <div className="flex gap-4">
                    <div className="w-10 h-10 rounded-xl bg-app-card shrink-0 flex items-center justify-center text-blue-400">
                      <Globe className="w-5 h-5" />
                    </div>
                    <div>
                      <h4 className="text-sm font-bold text-white uppercase tracking-wider mb-1">Resource Availability</h4>
                      <p className="text-sm text-app-subtext leading-relaxed">
                        Files are accessed through a local high-speed buffer, while Database connections utilize SSL/TLS encrypted tunneling for safety.
                      </p>
                    </div>
                  </div>
                </div>

                <div className="p-4 rounded-2xl bg-app-card border border-app-border">
                  <h4 className="text-[10px] font-black uppercase tracking-[0.2em] text-app-subtext mb-3">System Guidance</h4>
                  <p className="text-xs text-app-subtext italic">
                    "Choose 'Files' if your data is primarily static documents or CSV exports. Choose 'Database' for dynamic or production-level data streams."
                  </p>
                </div>
              </div>

              {/* Modal Footer */}
              <div className="p-6 border-t border-app-border bg-app-card/30 flex justify-end">
                <button
                  onClick={() => setShowInfo(false)}
                  className="px-6 py-2.5 bg-app-accent text-white font-bold rounded-xl text-sm hover:bg-app-accent/90 transition-all active:scale-95"
                >
                  Understood
                </button>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
