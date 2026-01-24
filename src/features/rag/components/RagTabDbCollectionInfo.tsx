import { Database } from "lucide-react";
import AnimatedContainer from "../../../shared/components/AnimatedContainer";

type Props = {
  tables: string[];
};

export function RagTabDbCollectionInfo({ tables }: Props) {
  return (
    <AnimatedContainer animation="fadeIn" className="p-6 border-b border-app-border bg-app-card/30">
      <div className="space-y-4">
        <div className="flex items-center gap-2">
          <Database className="w-5 h-5 text-purple-500" />
          <h3 className="text-base font-semibold text-app-text">Database Collection Configuration</h3>
        </div>
        <p className="text-sm text-app-text-muted">
          This collection queries a database connection with SQL-RAG. File import is not available for DB collections.
        </p>
        <div className="rounded-lg bg-app-card border border-app-border p-4">
          <h4 className="text-sm font-medium text-app-text mb-3">Selected Tables ({tables.length})</h4>
          {tables.length === 0 ? (
            <p className="text-xs text-app-text-muted">No tables selected for this collection</p>
          ) : (
            <div className="flex flex-wrap gap-2">
              {tables.map((table) => (
                <span
                  key={table}
                  className="inline-flex items-center px-2.5 py-1 rounded-md bg-purple-500/10 text-purple-500 text-xs font-medium">
                  <Database className="w-3 h-3 mr-1" />
                  {table}
                </span>
              ))}
            </div>
          )}
        </div>
        <div className="grid grid-cols-2 gap-4 mt-4">
          <div>
            <div className="text-[10px] text-app-subtext uppercase tracking-wider">Default Row Limit</div>
            <div className="text-sm text-app-text font-medium">50 rows max</div>
          </div>
          <div>
            <div className="text-[10px] text-app-subtext uppercase tracking-wider">LLM Policy</div>
            <div className="text-sm  font-medium text-red-500">Blocked (Local Only)</div>
          </div>
        </div>
      </div>
    </AnimatedContainer>
  );
}
