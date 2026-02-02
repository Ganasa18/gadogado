import { Database, Shield } from "lucide-react";
import AnimatedContainer from "../../../shared/components/AnimatedContainer";

type Props = {
  tables: string[];
  config: any;
};

export function RagTabDbCollectionInfo({ tables, config }: Props) {
  return (
    <AnimatedContainer animation="fadeIn" className="p-6 border-b border-app-border bg-app-card/30">
      <div className="space-y-5">
        <div className="flex items-center gap-2">
          <Database className="w-5 h-5 text-app-text" />
          <h3 className="text-base font-semibold text-app-text">Database Collection Configuration</h3>
        </div>
        <p className="text-sm text-app-text-muted">
          This collection queries a database connection with SQL-RAG. File import is not available for DB collections.
        </p>

        {/* Tables */}
        <div className="rounded-lg border border-app-border p-4">
          <h4 className="text-sm font-medium text-app-text mb-3">Selected Tables ({tables.length})</h4>
          {tables.length === 0 ? (
            <p className="text-xs text-app-text-muted">No tables selected for this collection</p>
          ) : (
            <div className="flex flex-wrap gap-2">
              {tables.map((table) => (
                <span
                  key={table}
                  className="inline-flex items-center px-2.5 py-1 rounded border border-app-border text-xs text-app-text">
                  <Database className="w-3 h-3 mr-1" />
                  {table}
                </span>
              ))}
            </div>
          )}
        </div>

        {/* Configuration Details */}
        {config && (
          <div className="rounded-lg border border-app-border p-4">
            <h4 className="text-sm font-medium text-app-text mb-3 flex items-center gap-2">
              <Shield className="w-4 h-4" />
              Collection Settings
            </h4>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <div className="text-[10px] text-app-subtext uppercase tracking-wider">Default Row Limit</div>
                <div className="text-sm text-app-text font-medium">{config.default_limit || 50} rows max</div>
              </div>
              <div>
                <div className="text-[10px] text-app-subtext uppercase tracking-wider">Max Row Limit</div>
                <div className="text-sm text-app-text font-medium">{config.max_limit || 200} rows</div>
              </div>
              <div>
                <div className="text-[10px] text-app-subtext uppercase tracking-wider">Profile ID</div>
                <div className="text-sm text-app-text font-medium">{config.allowlist_profile_id}</div>
              </div>
              <div>
                <div className="text-[10px] text-app-subtext uppercase tracking-wider">DB Connection ID</div>
                <div className="text-sm text-app-text font-medium">{config.db_conn_id}</div>
              </div>
              <div>
                <div className="text-[10px] text-app-subtext uppercase tracking-wider">External LLM</div>
                <div className="text-sm text-app-text font-medium capitalize">
                  {config.external_llm_policy === 'block' && 'Blocked (Local Only)'}
                  {config.external_llm_policy === 'allow' && 'Allowed'}
                  {config.external_llm_policy === 'local_only' && 'Local Only'}
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </AnimatedContainer>
  );
}
