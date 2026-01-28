import { Database } from "lucide-react";
import type { DbConnection } from "../../rag/types";
import { cn } from "../../../utils/cn";
import { ConnectionMenu } from "./ConnectionMenu";

interface ConnectionTableProps {
  connections: DbConnection[];
  onTest: (id: number) => void;
  onDeleteClick: (id: number, name: string) => void;
  onConfigureProfile: (connection: DbConnection) => void;
  onManageTables: (connection: DbConnection) => void;
  onEditConfig?: (connection: DbConnection) => void;
}

export function ConnectionTable({
  connections,
  onTest,
  onDeleteClick,
  onConfigureProfile,
  onManageTables,
  onEditConfig,
}: ConnectionTableProps) {
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
                <div className="flex justify-end">
                  <ConnectionMenu
                    connection={conn}
                    onTest={onTest}
                    onDeleteClick={onDeleteClick}
                    onConfigureProfile={onConfigureProfile}
                    onManageTables={onManageTables}
                    onEditConfig={onEditConfig}
                  />
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
