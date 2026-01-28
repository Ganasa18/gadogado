import { useState, useEffect } from "react";
import { useParams, useNavigate } from "react-router";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";
import {
  ChevronRight,
  Database,
  Search,
  Settings,
  Shield,
  Loader2,
  Check,
  AlertCircle,
  X,
  Save,
  Table as TableIcon,
} from "lucide-react";
import { cn } from "../../../utils/cn";
import type {
  ColumnInfo,
  DbConnection,
  TableInfo,
  DbAllowlistProfile,
} from "../../rag/types";
import {
  dbSaveConnectionConfig,
  dbGetConnectionConfig,
  dbSyncProfileTables,
} from "../../rag/api";

import { ProfileManagementModal } from "../components/ProfileManagementModal";
import { QueryTemplatesSection } from "../components/QueryTemplatesSection";
import { Pagination } from "../components/Pagination";

const TABLES_PER_PAGE = 10;
const COLUMNS_PER_PAGE = 15;

// Logging utility for debugging
const add_log = (category: string, message: string, data?: unknown) => {
  const timestamp = new Date().toISOString();
  const logEntry = `[${timestamp}] [${category}] ${message}`;
  if (data) {
    console.log(logEntry, data);
  } else {
    console.log(logEntry);
  }
};

// Load saved configuration for a connection from database
const loadSavedConfig = async (connId: number): Promise<{
  tables: Set<string>;
  profileId: number | null;
  selectedColumns: Map<string, Set<string>>
}> => {
  try {
    const config = await dbGetConnectionConfig(connId);
    return {
      tables: new Set(config.selected_tables || []),
      profileId: config.profile_id || null,
      selectedColumns: new Map(
        Object.entries(config.selected_columns || {})
          .map(([table, cols]) => [table, new Set(cols as string[])] as [string, Set<string>])
      )
    };
  } catch (err) {
    console.error("Failed to load saved config from database:", err);
  }
  return { tables: new Set(), profileId: null, selectedColumns: new Map() };
};

// Save configuration to database (including selected columns and sync to profile)
const saveTablesToStorage = async (
  connId: number,
  tables: Set<string>,
  profileId: number | null,
  selectedColumns: Map<string, Set<string>>
): Promise<void> => {
  try {
    // Save to connection config
    await dbSaveConnectionConfig(
      connId,
      profileId ?? 0,
      Array.from(tables),
      Object.fromEntries(
        Array.from(selectedColumns.entries())
          .map(([table, cols]) => [table, Array.from(cols)])
      )
    );

    // Sync tables to profile's allowed_tables with actual column names from schema
    // SECURITY: This queries the database schema to get explicit column lists
    // which prevents SQL injection by ensuring all columns are validated
    if (profileId) {
      await dbSyncProfileTables(profileId, connId, Array.from(tables));
    }
  } catch (err) {
    console.error("Failed to save config to database:", err);
    throw err;
  }
};

export default function TableSetupPage() {
  const { id } = useParams();
  const navigate = useNavigate();
  const connId = id ? parseInt(id) : null;

  const [connection, setConnection] = useState<DbConnection | null>(null);
  const [tables, setTables] = useState<TableInfo[]>([]);
  const [selectedTable, setSelectedTable] = useState<TableInfo | null>(null);
  const [profiles, setProfiles] = useState<DbAllowlistProfile[]>([]);
  const [selectedProfile, setSelectedProfile] = useState<DbAllowlistProfile | null>(null);
  const [allowedTables, setAllowedTables] = useState<Set<string>>(new Set());
  const [columns, setColumns] = useState<Map<string, ColumnInfo[]>>(new Map());
  const [selectedColumns, setSelectedColumns] = useState<Map<string, Set<string>>>(new Map());

  const [loading, setLoading] = useState(true);
  const [loadingTables, setLoadingTables] = useState(false);
  const [loadingColumns, setLoadingColumns] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [showProfileModal, setShowProfileModal] = useState(false);
  
  // Pagination State
  const [tablePage, setTablePage] = useState(1);
  const [columnPage, setColumnPage] = useState(1);

  useEffect(() => {
    if (connId) {
      loadData();
    } else {
      setError("No connection ID provided");
      setLoading(false);
    }
  }, [connId]);

  const loadData = async () => {
    if (!connId) return;
    add_log("TableSetup", "Starting data load", { connId });
    setLoading(true);
    setError(null);
    try {
      // Load connections to find the current one
      const conns = await invoke<DbConnection[]>("db_list_connections");
      add_log("TableSetup", "Loaded connections", { count: conns.length });
      const currentConn = conns.find((c) => c.id === connId);
      if (!currentConn) {
        add_log("TableSetup", "ERROR: Connection not found", { connId });
        setError("Connection not found");
        return;
      }
      setConnection(currentConn);
      add_log("TableSetup", "Found current connection", { name: currentConn.name, dbType: currentConn.db_type });

      // Load profiles
      const loadedProfiles = await reloadProfiles();

      // Load saved config from database (tables + profile + columns)
      const savedConfig = await loadSavedConfig(connId);
      add_log("TableSetup", "Loaded saved config", {
        tablesCount: savedConfig.tables.size,
        profileId: savedConfig.profileId,
        columnsTables: savedConfig.selectedColumns.size
      });
      setAllowedTables(savedConfig.tables);
      setSelectedColumns(savedConfig.selectedColumns);

      // Set selected profile from saved config if exists
      if (savedConfig.profileId) {
        const savedProfile = loadedProfiles.find((p) => p.id === savedConfig.profileId);
        if (savedProfile) {
          setSelectedProfile(savedProfile);
          add_log("TableSetup", "Restored saved profile", { id: savedProfile.id, name: savedProfile.name });
        }
      }

      // Load tables from database
      await fetchTables(currentConn.id);
    } catch (err) {
      console.error("Failed to load setup data:", err);
      add_log("TableSetup", "ERROR: Failed to load setup data", { error: err });
      setError("Failed to load configuration data");
    } finally {
      setLoading(false);
    }
  };

  const fetchTables = async (id: number) => {
    setLoadingTables(true);
    try {
      const dbTables = await invoke<TableInfo[]>("db_list_tables", { connId: id });
      setTables(dbTables);
      if (dbTables.length > 0 && !selectedTable) {
        setSelectedTable(dbTables[0]);
      }
    } catch (err) {
      console.error("Failed to fetch tables:", err);
    } finally {
      setLoadingTables(false);
    }
  };

  const fetchColumns = async (tableName: string): Promise<ColumnInfo[] | null> => {
    if (columns.has(tableName)) return columns.get(tableName) || null;
    if (!connId) return null;

    setLoadingColumns(prev => new Set(prev).add(tableName));
    try {
      const cols = await invoke<ColumnInfo[]>("db_list_columns", {
        connId,
        tableName
      });
      setColumns(prev => new Map(prev).set(tableName, cols));
      return cols;
    } catch (err) {
      console.error(`Failed to fetch columns for ${tableName}:`, err);
      return null;
    } finally {
      setLoadingColumns(prev => {
        const next = new Set(prev);
        next.delete(tableName);
        return next;
      });
    }
  };

  // Reset pagination when table changes
  useEffect(() => {
    if (selectedTable) {
      setColumnPage(1);
      fetchColumns(selectedTable.table_name);
    }
  }, [selectedTable]);

  // Reset table pagination when search query changes
  useEffect(() => {
    setTablePage(1);
  }, [searchQuery]);

  const handleToggleTable = async (tableName: string) => {
    const newAllowed = new Set(allowedTables);
    let newSelectedColumns = selectedColumns;

    if (newAllowed.has(tableName)) {
      newAllowed.delete(tableName);
    } else {
      newAllowed.add(tableName);
      // Fetch columns when table is added
      const cols = await fetchColumns(tableName);
      // Initialize selectedColumns with all columns (opt-out model) only when adding to allowlist
      if (cols && !selectedColumns.has(tableName)) {
        newSelectedColumns = new Map(selectedColumns);
        newSelectedColumns.set(tableName, new Set(cols.map(c => c.column_name)));
        setSelectedColumns(newSelectedColumns);
      }
    }
    setAllowedTables(newAllowed);

    // Auto-save to database (including selected profile and columns)
    if (connId) {
      await saveTablesToStorage(connId, newAllowed, selectedProfile?.id || null, newSelectedColumns);
    }
  };

  const handleSave = async () => {
    if (!connId) return;

    add_log("TableSetup", "Saving configuration", {
      connId,
      tablesCount: allowedTables.size,
      profileId: selectedProfile?.id,
      profileName: selectedProfile?.name,
      columnsTables: selectedColumns.size
    });

    // Save to database (including selected profile and columns)
    await saveTablesToStorage(connId, allowedTables, selectedProfile?.id || null, selectedColumns);

    // Show success feedback
    console.log(`Saved ${allowedTables.size} tables for connection ${connId}`);
    add_log("TableSetup", "Configuration saved successfully");

    // Navigate back after save
    setTimeout(() => {
      navigate("/database");
    }, 500);
  };

  const reloadProfiles = async () => {
    try {
      const profileList = await invoke<DbAllowlistProfile[]>("db_list_allowlist_profiles");
      setProfiles(profileList);
      if (profileList.length > 0 && !selectedProfile) {
        setSelectedProfile(profileList[0]);
      }
      return profileList;
    } catch (err) {
      console.error("Failed to reload profiles:", err);
      return [];
    }
  };

  const filteredTables = tables.filter((t) =>
    t.table_name.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const paginatedTables = filteredTables.slice(
    (tablePage - 1) * TABLES_PER_PAGE,
    tablePage * TABLES_PER_PAGE
  );

  const currentTableColumns = columns.get(selectedTable?.table_name || "") || [];
  const paginatedColumns = currentTableColumns.slice(
    (columnPage - 1) * COLUMNS_PER_PAGE,
    columnPage * COLUMNS_PER_PAGE
  );

  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center bg-app-bg text-app-text min-h-screen">
        <div className="flex flex-col items-center gap-4">
          <Loader2 className="w-10 h-10 animate-spin text-app-accent" />
          <p className="text-app-subtext animate-pulse font-medium">Loading setup configuration...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center bg-app-bg text-app-text min-h-screen gap-6">
        <div className="w-16 h-16 rounded-full bg-destructive/10 flex items-center justify-center text-destructive">
          <AlertCircle className="w-8 h-8" />
        </div>
        <div className="text-center space-y-2">
          <h2 className="text-xl font-bold">Configuration Error</h2>
          <p className="text-app-subtext">{error}</p>
        </div>
        <button 
          onClick={() => navigate("/database")}
          className="px-6 py-2 bg-app-card border border-app-border rounded-lg text-sm font-bold hover:bg-app-border/40 transition-all"
        >
          Back to Connections
        </button>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-screen bg-app-bg text-app-text overflow-hidden">
      {/* Header */}
      <header className="px-8 py-6 border-b border-app-border bg-app-panel/50 backdrop-blur-md flex justify-between items-center shrink-0">
        <div className="space-y-1">
          <div className="flex items-center gap-2 text-[10px] font-bold uppercase tracking-widest text-app-subtext">
            <span>Config</span>
            <ChevronRight className="w-3 h-3" />
            <span>Table Setup</span>
            <ChevronRight className="w-3 h-3 text-app-accent" />
            <span className="text-app-accent">Profile</span>
          </div>
          <h1 className="text-2xl font-bold tracking-tight">Table Profile & Column Setup</h1>
        </div>
        <div className="flex items-center gap-3">
          <button 
            onClick={() => navigate("/database")}
            className="px-5 py-2 text-sm font-semibold text-app-subtext hover:text-app-text transition-colors flex items-center gap-2"
          >
            <X className="w-4 h-4" />
            Discard
          </button>
          <button
            onClick={handleSave}
            className="px-6 py-2.5 bg-app-accent text-white rounded-lg text-sm font-bold shadow-lg shadow-app-accent/20 hover:bg-app-accent/90 transition-all hover:scale-[1.02] active:scale-[0.98] flex items-center gap-2"
          >
            <Save className="w-4 h-4" />
            Save Configuration
          </button>
        </div>
      </header>

      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar - Database Tables */}
        <aside className="w-80 border-r border-app-border bg-app-panel/30 flex flex-col">
          <div className="p-6 space-y-4">
            <div>
              <h2 className="text-xs font-bold uppercase tracking-wider text-app-text mb-1">Database Tables</h2>
              <p className="text-[11px] text-app-subtext">Select table to configure</p>
            </div>
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-app-subtext" />
              <input
                type="text"
                placeholder="Search tables..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full bg-app-card/40 border border-app-border rounded-lg pl-10 pr-4 py-2 text-sm focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none transition-all"
              />
            </div>
          </div>

          <div className="flex-1 min-h-0 flex flex-col">
            <div className="flex-1 overflow-y-auto px-3 space-y-1 custom-scrollbar">
              {loadingTables ? (
                <div className="flex justify-center py-10">
                  <Loader2 className="w-6 h-6 animate-spin text-app-subtext/40" />
                </div>
              ) : filteredTables.length === 0 ? (
                <div className="text-center py-10 px-4">
                  <p className="text-xs text-app-subtext italic">No tables found</p>
                </div>
              ) : (
                paginatedTables.map((table) => (
                  <button
                    key={table.table_name}
                    onClick={() => setSelectedTable(table)}
                    className={cn(
                      "w-full flex items-center gap-3 p-3 rounded-xl transition-all group relative",
                      selectedTable?.table_name === table.table_name
                        ? "bg-app-accent/10 text-app-accent border border-app-accent/20 shadow-sm"
                        : "hover:bg-app-card/30 text-app-subtext hover:text-app-text border border-transparent"
                    )}
                  >
                    <AnimatePresence>
                      {selectedTable?.table_name === table.table_name && (
                        <motion.div
                          layoutId="active-indicator"
                          initial={{ opacity: 0 }}
                          animate={{ opacity: 1 }}
                          exit={{ opacity: 0 }}
                          className="absolute left-0 w-1 h-6 bg-app-accent rounded-r-full"
                        />
                      )}
                    </AnimatePresence>
                    <div className={cn(
                      "w-10 h-10 rounded-lg flex items-center justify-center shrink-0 transition-colors",
                      selectedTable?.table_name === table.table_name
                        ? "bg-app-accent text-white"
                        : "bg-app-card group-hover:bg-app-border/40 text-app-subtext group-hover:text-app-text"
                    )}>
                      <TableIcon className="w-5 h-5" />
                    </div>
                    <div className="text-left overflow-hidden">
                      <div className="text-sm font-bold truncate">{table.table_name}</div>
                      <div className="text-[10px] opacity-70 truncate">
                        {table.row_count?.toLocaleString() ?? "???"} rows • {table.table_schema ?? "public"}
                      </div>
                    </div>
                    {allowedTables.has(table.table_name) && (
                      <div className="ml-auto w-2 h-2 rounded-full bg-app-success shadow-[0_0_8px_rgba(16,185,129,0.5)]" />
                    )}
                  </button>
                ))
              )}
            </div>
            
            <Pagination 
              currentPage={tablePage}
              totalItems={filteredTables.length}
              itemsPerPage={TABLES_PER_PAGE}
              onPageChange={setTablePage}
              className="px-4 border-t border-app-border bg-app-panel/50"
            />
          </div>
        </aside>

        {/* Main Content Area */}
        <main className="flex-1 flex flex-col bg-app-bg relative min-h-0">
          <AnimatePresence mode="wait">
            {selectedTable ? (
              <motion.div 
                key={selectedTable?.table_name || "empty"}
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -10 }}
                className="flex-1 flex flex-col min-h-0"
              >
                {/* Scrollable Container */}
                <div className="flex-1 overflow-y-auto custom-scrollbar p-8">
                  <div className="space-y-8 max-w-[1400px] mx-auto">
                    {/* Table Header Section */}
                    <div className="flex justify-between items-end">
                      <div className="space-y-1">
                        <h2 className="text-3xl font-bold text-app-text flex items-center gap-3">
                          Profile Setup: <span className="text-app-accent">{selectedTable?.table_name}</span>
                        </h2>
                        <p className="text-app-subtext max-w-2xl">
                          Configure schema mapping and column selection for RAG indexing. 
                          Allowed tables will be indexed in 15-minute intervals.
                        </p>
                      </div>
                      <div className="text-right space-y-1">
                        <div className="text-[10px] font-bold uppercase tracking-widest text-app-subtext">Data Type</div>
                        <div className="text-sm font-mono text-app-text flex items-center gap-2 justify-end">
                          <span className="w-2 h-2 rounded-full bg-app-success" />
                          {connection?.db_type === 'postgres' ? 'PostgreSQL V15' : 'SQLite 3.x'}
                        </div>
                      </div>
                    </div>

                    {/* Profile Selector Section */}
                    <div className="grid grid-cols-12 gap-8">
                      <div className="col-span-8 space-y-6">
                        {/* Access Toggle Card */}
                        <div 
                          onClick={() => selectedTable && handleToggleTable(selectedTable.table_name)}
                          className={cn(
                            "p-6 rounded-2xl border transition-all cursor-pointer group",
                            allowedTables.has(selectedTable.table_name)
                              ? "bg-app-success/5 border-app-success/20 hover:border-app-success/40"
                              : "bg-app-panel/40 border-app-border hover:border-app-subtext/30"
                          )}
                        >
                          <div className="flex items-center justify-between">
                            <div className="flex items-center gap-4">
                              <div className={cn(
                                "w-12 h-12 rounded-xl flex items-center justify-center transition-colors",
                                allowedTables.has(selectedTable.table_name)
                                  ? "bg-app-success text-white shadow-lg shadow-app-success/20"
                                  : "bg-app-card text-app-subtext"
                              )}>
                                <Shield className="w-6 h-6" />
                              </div>
                              <div>
                                <h3 className="font-bold text-app-text">Table Access Status</h3>
                                <p className="text-sm text-app-subtext">Click to toggle allowlist status for this table</p>
                              </div>
                            </div>
                            <div className={cn(
                              "px-4 py-1.5 rounded-full text-xs font-bold uppercase tracking-wider transition-all",
                              allowedTables.has(selectedTable.table_name)
                                ? "bg-app-success/20 text-app-success border border-app-success/30"
                                : "bg-app-subtext/10 text-app-subtext border border-app-border"
                            )}>
                              {allowedTables.has(selectedTable.table_name) ? "ALLOWED" : "DENIED"}
                            </div>
                          </div>
                        </div>

                        {/* Column Selection for Active Table */}
                        <div className="bg-app-panel/40 border border-app-border rounded-2xl overflow-hidden shadow-sm flex flex-col">
                          <div className="px-6 py-4 bg-app-card/30 border-b border-app-border flex justify-between items-center text-[10px] font-bold uppercase tracking-widest text-app-subtext">
                            <div className="flex items-center gap-4">
                              <Check className="w-3.5 h-3.5" />
                              <span>Column Name</span>
                            </div>
                            <div className="flex items-center gap-20 mr-20">
                              <span>Data Type</span>
                              <span>Nullable</span>
                            </div>
                            <span>Actions</span>
                          </div>

                          <div className="divide-y divide-app-border/40">
                            {loadingColumns.has(selectedTable?.table_name || "") ? (
                              <div className="p-12 text-center space-y-3">
                                <Loader2 className="w-8 h-8 mx-auto animate-spin text-app-accent" />
                                <p className="text-sm text-app-subtext">Loading columns...</p>
                              </div>
                            ) : currentTableColumns.length === 0 ? (
                              <div className="p-12 text-center space-y-3">
                                <AlertCircle className="w-12 h-12 mx-auto text-app-subtext/40" />
                                <p className="text-sm text-app-text">No columns found</p>
                                <p className="text-xs text-app-subtext">Could not fetch table schema</p>
                              </div>
                            ) : (
                              paginatedColumns.map((col, idx) => (
                                <div
                                  key={col.column_name}
                                  className="px-6 py-3 flex items-center justify-between hover:bg-app-card/30 transition-colors group"
                                >
                                  <div className="flex items-center gap-4 flex-1">
                                    <div className="w-5 flex items-center justify-center text-app-subtext/40 text-xs font-mono">
                                      {(columnPage - 1) * COLUMNS_PER_PAGE + idx + 1}
                                    </div>
                                    <div className="flex-1">
                                      <code className="text-sm font-mono text-app-text">{col.column_name}</code>
                                      <div className="text-[10px] text-app-subtext mt-0.5 flex items-center gap-2">
                                        <span className="px-1.5 py-0.5 bg-app-border/50 rounded">{col.data_type}</span>
                                        {col.is_primary_key && (
                                          <span className="text-app-accent">PK</span>
                                        )}
                                        {col.is_nullable && (
                                          <span className="text-app-subtext">NULL</span>
                                        )}
                                      </div>
                                    </div>
                                  </div>
                                  <label className="flex items-center gap-2 cursor-pointer">
                                    <input
                                      type="checkbox"
                                      checked={selectedColumns.get(selectedTable?.table_name || "")?.has(col.column_name) || false}
                                      onChange={async (e) => {
                                        const tableName = selectedTable?.table_name;
                                        if (!tableName) return;

                                        const newMap = new Map(selectedColumns);
                                        const tableCols = newMap.get(tableName) || new Set();
                                        if (e.target.checked) {
                                          tableCols.add(col.column_name);
                                        } else {
                                          tableCols.delete(col.column_name);
                                        }
                                        newMap.set(tableName, tableCols);
                                        setSelectedColumns(newMap);

                                        // Auto-save to database
                                        if (connId) {
                                          await saveTablesToStorage(connId, allowedTables, selectedProfile?.id || null, newMap);
                                        }
                                      }}
                                      className="w-4 h-4 rounded border-app-border text-app-accent focus:ring-app-accent/50"
                                    />
                                    <span className="text-xs text-app-subtext group-hover:text-app-text">
                                      {col.is_primary_key ? "Primary Key" : "Include"}
                                    </span>
                                  </label>
                                </div>
                              ))
                            )}
                          </div>
                          
                          <Pagination 
                            currentPage={columnPage}
                            totalItems={currentTableColumns.length}
                            itemsPerPage={COLUMNS_PER_PAGE}
                            onPageChange={setColumnPage}
                            className="px-4 py-2 bg-app-card/20 border-t border-app-border"
                          />
                        </div>
                      </div>

                      {/* Right Column - Secondary Settings */}
                      <div className="col-span-4 space-y-6">
                        <div className="bg-app-panel/40 border border-app-border rounded-2xl p-6 space-y-6">
                          <div>
                            <div className="flex items-center justify-between mb-4">
                              <h3 className="text-[10px] font-bold uppercase tracking-widest text-app-subtext">Security Profile</h3>
                              <button
                                onClick={() => setShowProfileModal(true)}
                                className="text-xs text-app-accent hover:text-app-accent/80 flex items-center gap-1"
                              >
                                <Settings className="w-3 h-3" />
                                Manage
                              </button>
                            </div>
                            <div className="relative">
                              <select
                                value={selectedProfile?.id || ""}
                                onChange={async (e) => {
                                  const p = profiles.find(x => x.id === parseInt(e.target.value));
                                  setSelectedProfile(p || null);
                                  // Save profile selection to database (including columns)
                                  if (connId && p) {
                                    await saveTablesToStorage(connId, allowedTables, p.id, selectedColumns);
                                  }
                                }}
                                className="w-full bg-app-card border border-app-border rounded-xl px-4 py-3 text-sm font-semibold text-app-text focus:ring-2 focus:ring-app-accent/50 focus:border-app-accent outline-none appearance-none cursor-pointer"
                              >
                                {profiles.map(p => (
                                  <option key={p.id} value={p.id}>{p.name}</option>
                                ))}
                              </select>
                              <ChevronRight className="absolute right-4 top-1/2 -translate-y-1/2 w-4 h-4 text-app-subtext pointer-events-none rotate-90" />
                            </div>
                          </div>

                          {/* Configuration Preview */}
                          <div className="bg-app-panel/40 border border-app-border rounded-2xl p-5 space-y-4">
                            <h3 className="text-[10px] font-bold uppercase tracking-widest text-app-subtext">
                              Preview
                            </h3>

                            {/* Summary Stats */}
                            <div className="grid grid-cols-2 gap-3">
                              <div className="bg-app-card/50 rounded-lg p-3 text-center">
                                <div className="text-2xl font-bold text-app-accent">
                                  {allowedTables.size}
                                </div>
                                <div className="text-[10px] text-app-subtext uppercase tracking-wider">
                                  Tables
                                </div>
                              </div>
                              <div className="bg-app-card/50 rounded-lg p-3 text-center">
                                <div className="text-2xl font-bold text-app-success">
                                  {Array.from(allowedTables).reduce((sum, tableName) => {
                                    const cols = selectedColumns.get(tableName);
                                    return sum + (cols?.size || 0);
                                  }, 0)}
                                </div>
                                <div className="text-[10px] text-app-subtext uppercase tracking-wider">
                                  Columns
                                </div>
                              </div>
                            </div>

                            {/* Tables & Columns List */}
                            <div className="space-y-2 max-h-48 overflow-y-auto custom-scrollbar">
                              {Array.from(allowedTables)
                                .filter(tableName => {
                                  const cols = selectedColumns.get(tableName);
                                  return cols && cols.size > 0;
                                })
                                .sort()
                                .map(tableName => {
                                  const cols = selectedColumns.get(tableName);
                                  if (!cols || cols.size === 0) return null;

                                  return (
                                    <div key={tableName} className="bg-app-card/30 rounded-lg p-2.5">
                                      <div className="flex items-center justify-between mb-1.5">
                                        <code className="text-xs font-mono text-app-text font-semibold">
                                          {tableName}
                                        </code>
                                        <span className="text-[10px] text-app-subtext">
                                          {cols.size} cols
                                        </span>
                                      </div>
                                      <div className="flex flex-wrap gap-1">
                                        {Array.from(cols)
                                          .sort()
                                          .slice(0, 5)
                                          .map(col => (
                                            <span
                                              key={col}
                                              className="px-2 py-0.5 bg-app-border/50 rounded text-[9px] text-app-subtext font-mono"
                                            >
                                              {col}
                                            </span>
                                          ))}
                                        {cols.size > 5 && (
                                          <span className="px-2 py-0.5 bg-app-border/30 rounded text-[9px] text-app-subtext font-mono">
                                            +{cols.size - 5}
                                          </span>
                                        )}
                                      </div>
                                    </div>
                                  );
                                })}
                              {allowedTables.size === 0 && (
                                <div className="text-center py-4 text-app-subtext text-xs">
                                  No tables selected
                                </div>
                              )}
                            </div>

                            {/* Profile Info */}
                            {selectedProfile && (
                              <div className="pt-3 border-t border-app-border">
                                <div className="text-[10px] text-app-subtext mb-1">Using Profile</div>
                                <div className="text-xs font-semibold text-app-text truncate">
                                  {selectedProfile.name}
                                </div>
                              </div>
                            )}
                          </div>

                          <div className="space-y-4">
                            <h3 className="text-[10px] font-bold uppercase tracking-widest text-app-subtext">Active Policies</h3>
                            <div className="space-y-3">
                              {[
                                { label: "Max Limit", value: "200 rows" },
                                { label: "Join Queries", value: "Restricted", accent: true },
                                { label: "SSL Enforcement", value: "Required" },
                              ].map((item, i) => (
                                <div key={i} className="flex items-center justify-between text-xs">
                                  <span className="text-app-subtext">{item.label}</span>
                                  <span className={cn("font-bold", item.accent ? "text-app-accent" : "text-app-text")}>
                                    {item.value}
                                  </span>
                                </div>
                              ))}
                            </div>
                          </div>

                          <div className="pt-4 border-t border-app-border">
                            <p className="text-[11px] text-app-subtext leading-relaxed">
                              Applied profile restricts queries to indexed columns only. Keywords like password are auto-redacted.
                            </p>
                          </div>
                        </div>

                        <div className="bg-app-accent/10 border border-app-accent/20 rounded-2xl p-5 flex gap-4">
                          <AlertCircle className="w-5 h-5 text-app-accent shrink-0" />
                          <p className="text-[11px] text-app-accent leading-relaxed">
                            Changes to table access will trigger a metadata re-index for all associated RAG collections.
                          </p>
                        </div>
                      </div>
                    </div>

                    {/* Query Templates Section - Full Width */}
                    <QueryTemplatesSection
                      profileId={selectedProfile?.id || null}
                      availableTables={Array.from(allowedTables)}
                    />
                  </div>
                </div>
              </motion.div>
            ) : (
              <div className="flex-1 flex items-center justify-center">
                <div className="text-center space-y-4 opacity-40">
                  <Database className="w-16 h-16 mx-auto" />
                  <p>Select a table to configure its profile</p>
                </div>
              </div>
            )}
          </AnimatePresence>
        </main>
      </div>

      {/* Profile Management Modal */}
      <ProfileManagementModal
        isOpen={showProfileModal}
        onClose={() => setShowProfileModal(false)}
        onProfileChange={reloadProfiles}
        currentProfiles={profiles}
      />
    </div>
  );
}
