import { useMemo, type ReactNode } from "react";
import {
  AlertCircle,
  ArrowRight,
  ChevronDown,
  Database,
  FileText,
  Loader2,
  Plus,
  Settings,
  Shield,
  Trash2,
  X,
} from "lucide-react";
import type {
  CollectionKind,
  DbAllowlistProfile,
  DbConnection,
  RagCollection,
} from "../types";

type Props = {
  collections: RagCollection[];
  selectedCollectionId: number | null;
  onSelectCollection: (id: number) => void;
  onDeleteCollection: (id: number) => void;

  showCreateForm: boolean;
  onShowCreateForm: (v: boolean) => void;

  collectionKind: CollectionKind;
  onChangeCollectionKind: (v: CollectionKind) => void;

  newCollectionName: string;
  onChangeNewCollectionName: (v: string) => void;
  newCollectionDescription: string;
  onChangeNewCollectionDescription: (v: string) => void;

  isCreatingCollection: boolean;
  onCreateCollection: () => void;
  onCancelCreate: () => void;

  // DB
  dbConnections: DbConnection[];
  dbConnId: number | null;
  onChangeDbConnId: (v: number | null) => void;
  allowlistProfiles: DbAllowlistProfile[];
  allowlistProfileId: number;
  onChangeAllowlistProfileId: (id: number) => void;
  availableTables: string[];
  selectedTables: string[];
  onChangeSelectedTables: (tables: string[]) => void;
  isLoadingDbData: boolean;
  onOpenDbConnections: () => void;
};

type CollectionKindInfo = {
  value: CollectionKind;
  label: string;
  description: string;
  icon: ReactNode;
};

export function RagTabSidebar(props: Props) {
  const {
    collections,
    selectedCollectionId,
    onSelectCollection,
    onDeleteCollection,
    showCreateForm,
    onShowCreateForm,
    collectionKind,
    onChangeCollectionKind,
    newCollectionName,
    onChangeNewCollectionName,
    newCollectionDescription,
    onChangeNewCollectionDescription,
    isCreatingCollection,
    onCreateCollection,
    onCancelCreate,
    dbConnections,
    dbConnId,
    onChangeDbConnId,
    allowlistProfiles,
    allowlistProfileId,
    onChangeAllowlistProfileId,
    availableTables,
    selectedTables,
    onChangeSelectedTables,
    isLoadingDbData,
    onOpenDbConnections,
  } = props;

  const collectionKinds = useMemo<CollectionKindInfo[]>(
    () => [
      {
        value: "Files",
        label: "Files",
        description: "Upload PDF, DOCX, CSV, TXT, or import from web",
        icon: <FileText className="w-5 h-5" />,
      },
      {
        value: "Db",
        label: "Database",
        description: "Connect to external database (PostgreSQL/SQLite)",
        icon: <Database className="w-5 h-5" />,
      },
    ],
    [],
  );

  return (
    <aside className="w-80 border-r border-app-border flex flex-col">
      <div className="p-4 border-b border-app-border">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <Database className="w-5 h-5 text-app-accent" />
            <h2 className="text-lg font-semibold text-app-text">Collections</h2>
          </div>
          {!showCreateForm && (
            <button
              onClick={() => onShowCreateForm(true)}
              className="flex items-center gap-1 px-3 py-1.5 text-sm bg-app-accent text-white rounded-md hover:opacity-90 transition-opacity">
              <Plus className="w-4 h-4" />
              New
            </button>
          )}
        </div>

        {showCreateForm && (
          <div className="space-y-3">
            <div>
              <label className="text-[10px] text-app-subtext block mb-2 uppercase tracking-wider">
                Collection Type
              </label>
              <div className="grid grid-cols-2 gap-2">
                {collectionKinds.map((kind) => (
                  <button
                    key={kind.value}
                    type="button"
                    onClick={() => {
                      onChangeCollectionKind(kind.value);
                      onChangeDbConnId(null);
                      onChangeSelectedTables([]);
                    }}
                    className={`p-3 border rounded-lg text-left transition-all ${
                      collectionKind === kind.value
                        ? "border-app-accent bg-app-accent/10"
                        : "border-app-border bg-app-card hover:border-app-accent/40"
                    }`}>
                    <div className="flex justify-between items-center gap-2 mb-1">
                      {kind.icon}
                      <span className="text-sm font-medium text-app-text">
                        {kind.label}
                      </span>
                    </div>
                    <p className="text-[10px] text-app-text-muted">
                      {/* {kind.description} */}
                    </p>
                  </button>
                ))}
              </div>
            </div>

            {collectionKind === "Db" && (
              <div className="flex items-start gap-2 p-3 bg-yellow-500/10 border border-yellow-500/30 rounded-lg">
                <Shield className="w-4 h-4 text-yellow-500 mt-0.5 shrink-0" />
                <div className="text-xs text-yellow-600 dark:text-yellow-500">
                  <strong>Important:</strong> DB Collections are specialized for
                  database queries only and cannot be used with files (PDF, CSV,
                  etc.).
                </div>
              </div>
            )}

            <div>
              <label className="text-[10px] text-app-subtext block mb-1 uppercase tracking-wider">
                Name
              </label>
              <input
                value={newCollectionName}
                onChange={(e) => onChangeNewCollectionName(e.target.value)}
                placeholder="My Collection"
                className="w-full bg-app-card border border-app-border rounded-md px-3 py-2 text-sm outline-none focus:border-app-accent focus:ring-2 focus:ring-app-accent/20 transition-all"
              />
            </div>

            <div>
              <label className="text-[10px] text-app-subtext block mb-1 uppercase tracking-wider">
                Description (Optional)
              </label>
              <textarea
                value={newCollectionDescription}
                onChange={(e) =>
                  onChangeNewCollectionDescription(e.target.value)
                }
                placeholder="What's this collection for?"
                rows={2}
                className="w-full bg-app-card border border-app-border rounded-md px-3 py-2 text-sm outline-none focus:border-app-accent focus:ring-2 focus:ring-app-accent/20 transition-all resize-none"
              />
            </div>

            {collectionKind === "Db" && (
              <>
                <div>
                  <label className="text-[10px] text-app-subtext block mb-1 uppercase tracking-wider">
                    Security Profile
                  </label>
                  <div className="relative group">
                    <select
                      value={allowlistProfileId}
                      onChange={(e) => {
                        const id = parseInt(e.target.value);
                        onChangeAllowlistProfileId(id);
                        onChangeDbConnId(null);
                        onChangeSelectedTables([]);
                      }}
                      className="w-full appearance-none bg-app-card border border-app-border text-app-text text-sm rounded-lg px-4 py-2.5 outline-none focus:border-emerald-500/50 transition-colors cursor-pointer">
                      {allowlistProfiles.map((profile) => {
                        // Count tables in this profile's rules
                        const tableCount = (() => {
                          try {
                            const rules = JSON.parse(profile.rules_json);
                            return Object.keys(rules.allowed_tables || {})
                              .length;
                          } catch {
                            return 0;
                          }
                        })();

                        return (
                          <option key={profile.id} value={profile.id}>
                            {profile.name} ({tableCount} tables) -{" "}
                            {profile.description || "No description"}
                          </option>
                        );
                      })}
                    </select>
                    <ChevronDown className="absolute right-3 top-3 w-4 h-4 text-app-subtext pointer-events-none group-hover:text-emerald-500 transition-colors" />
                  </div>
                  <p className="text-[10px] text-app-text-muted mt-1">
                    Controls which tables, columns, and filters are allowed
                  </p>
                </div>

                {/* Warning: No tables in profile */}
                {allowlistProfileId && availableTables.length === 0 && (
                  <div className="flex items-start gap-2 p-3 bg-amber-500/10 border border-amber-500/30 rounded-lg">
                    <AlertCircle className="w-4 h-4 text-amber-500 mt-0.5 shrink-0" />
                    <div className="text-xs text-amber-600 dark:text-amber-500">
                      <strong>No tables configured</strong> for this profile. Go
                      to{" "}
                      <button
                        onClick={onOpenDbConnections}
                        className="underline hover:text-amber-500">
                        Database Settings
                      </button>{" "}
                      to setup tables first.
                    </div>
                  </div>
                )}

                <div>
                  <label className="text-[10px] text-app-subtext block mb-1 uppercase tracking-wider">
                    Database Connection
                  </label>
                  <div className="flex items-center gap-2 mb-4">
                    <div className="relative group flex-1">
                      <select
                        value={dbConnId ?? ""}
                        onChange={(e) =>
                          onChangeDbConnId(
                            e.target.value ? parseInt(e.target.value) : null,
                          )
                        }
                        className="w-full appearance-none bg-app-card border border-app-border text-app-text text-sm rounded-lg px-4 py-2.5 outline-none focus:border-emerald-500/50 transition-colors cursor-pointer">
                        <option value="">Select a connection...</option>
                        {dbConnections.map((conn) => (
                          <option key={conn.id} value={conn.id}>
                            {conn.name} ({conn.db_type.toUpperCase()})
                          </option>
                        ))}
                      </select>
                      <ChevronDown className="absolute right-3 top-3 w-4 h-4 text-app-subtext pointer-events-none group-hover:text-emerald-500 transition-colors" />
                    </div>
                    <button
                      onClick={onOpenDbConnections}
                      className="p-2 border border-app-border rounded-md text-app-text-muted hover:text-app-text hover:border-app-accent/50 transition-all"
                      title="Configure new connection">
                      <Settings className="w-4 h-4" />
                    </button>
                  </div>
                  {dbConnections.length === 0 && !isLoadingDbData && (
                    <p className="text-xs text-app-text-muted">
                      No connections configured.{" "}
                      <button
                        onClick={onOpenDbConnections}
                        className="text-app-accent hover:underline">
                        Create one →
                      </button>
                    </p>
                  )}
                </div>

                {dbConnId && availableTables.length > 0 && (
                  <div>
                    <label className="text-[10px] text-app-subtext block mb-1 uppercase tracking-wider">
                      Select Tables
                      <span className="ml-1 text-app-accent">
                        ({selectedTables.length} selected)
                      </span>
                    </label>
                    <p className="text-[10px] text-app-text-muted mb-2">
                      Choose which tables this collection can query
                    </p>
                    <div className="border border-app-border rounded-lg p-2 max-h-32 overflow-y-auto bg-app-card">
                      {availableTables.map((table) => (
                        <label
                          key={table}
                          className="flex items-center gap-2 py-1 px-2 hover:bg-app-bg rounded cursor-pointer">
                          <input
                            type="checkbox"
                            checked={selectedTables.includes(table)}
                            onChange={(e) => {
                              if (e.currentTarget.checked) {
                                onChangeSelectedTables([
                                  ...selectedTables,
                                  table,
                                ]);
                              } else {
                                onChangeSelectedTables(
                                  selectedTables.filter((t) => t !== table),
                                );
                              }
                            }}
                            className="rounded border-app-border"
                          />
                          <span className="text-xs text-app-text">{table}</span>
                        </label>
                      ))}
                    </div>
                    {selectedTables.length === 0 && (
                      <p className="text-[10px] text-red-500 mt-1">
                        At least one table must be selected
                      </p>
                    )}
                  </div>
                )}
              </>
            )}

            <div className="flex gap-2">
              <button
                onClick={onCreateCollection}
                disabled={
                  isCreatingCollection ||
                  !newCollectionName.trim() ||
                  (collectionKind === "Db" &&
                    (!dbConnId || selectedTables.length === 0))
                }
                className="flex-1 flex items-center justify-center gap-2 px-3 py-2 bg-app-accent text-white rounded-md text-sm hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed transition-opacity">
                {isCreatingCollection ? (
                  <Loader2 className="w-4 h-4 animate-spin" />
                ) : (
                  <ArrowRight className="w-4 h-4" />
                )}
                Create {collectionKind === "Db" ? "DB " : ""}Collection
              </button>
              <button
                onClick={onCancelCreate}
                className="flex items-center justify-center px-3 py-2 border border-app-border rounded-md text-sm text-app-text-muted hover:text-app-text transition-colors">
                <X className="w-4 h-4" />
              </button>
            </div>
          </div>
        )}
      </div>

      <div className="flex-1 overflow-y-auto p-4 space-y-2">
        {collections.length === 0 ? (
          <div className="text-sm text-app-text-muted">
            No collections yet. Create one to start importing.
          </div>
        ) : (
          collections.map((collection) => (
            <button
              key={collection.id}
              onClick={() => onSelectCollection(collection.id)}
              className={`w-full text-left p-3 rounded-lg border transition-all ${
                selectedCollectionId === collection.id
                  ? "border-app-accent bg-app-accent/10"
                  : "border-app-border bg-app-card hover:border-app-accent/40"
              }`}>
              <div className="flex items-start justify-between gap-3">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium text-app-text truncate">
                      {collection.name}
                    </span>
                    <span
                      className={`text-[10px] px-1.5 py-0.5 rounded shrink-0 ${
                        collection.kind === "Db"
                          ? "bg-app-accent/10 text-app-accent"
                          : "bg-app-card border border-app-border/40 text-app-subtext"
                      }`}>
                      {collection.kind === "Db" ? "DB" : "Files"}
                    </span>
                  </div>
                  {collection.description && (
                    <div className="text-xs text-app-text-muted mt-1">
                      {collection.description}
                    </div>
                  )}
                  <div className="text-[10px] text-app-text-muted mt-2">
                    Added{" "}
                    {new Date(collection.created_at).toLocaleDateString(
                      undefined,
                      {
                        month: "short",
                        day: "numeric",
                        year: "numeric",
                      },
                    )}
                  </div>
                </div>
                <button
                  onClick={(event) => {
                    event.stopPropagation();
                    void onDeleteCollection(collection.id);
                  }}
                  className="p-1.5 text-app-text-muted hover:text-red-500 transition-colors"
                  title="Delete collection">
                  <Trash2 className="w-4 h-4" />
                </button>
              </div>
            </button>
          ))
        )}
      </div>
    </aside>
  );
}
