import type { ApiKeyValueRow } from "../../hooks/useApiRequestBuilder";

type ApiKeyValueTableProps = {
  rows: ApiKeyValueRow[];
  emptyLabel: string;
  headerSuggestions?: string[];
  onAdd: () => void;
  onUpdate: (id: string, updates: Partial<ApiKeyValueRow>) => void;
  onRemove: (id: string) => void;
};

export function ApiKeyValueTable({
  rows,
  emptyLabel,
  headerSuggestions,
  onAdd,
  onUpdate,
  onRemove,
}: ApiKeyValueTableProps) {
  const datalistId = headerSuggestions ? "qa-header-keys" : undefined;

  return (
    <div className="space-y-2">
      {headerSuggestions && (
        <datalist id={datalistId}>
          {headerSuggestions.map((header) => (
            <option key={header} value={header} />
          ))}
        </datalist>
      )}
      {rows.map((row) => (
        <div
          key={row.id}
          className="grid grid-cols-[auto_1fr_1fr_auto] gap-2 items-center">
          <input
            type="checkbox"
            checked={row.enabled}
            onChange={(event) =>
              onUpdate(row.id, { enabled: event.target.checked })
            }
          />
          <input
            className="bg-[#181818] border border-app-border rounded p-2 text-xs"
            placeholder={emptyLabel}
            list={datalistId}
            value={row.key}
            onChange={(event) => onUpdate(row.id, { key: event.target.value })}
          />
          <input
            className="bg-[#181818] border border-app-border rounded p-2 text-xs"
            placeholder="value"
            value={row.value}
            onChange={(event) => onUpdate(row.id, { value: event.target.value })}
          />
          <button
            type="button"
            onClick={() => onRemove(row.id)}
            className="text-[10px] text-red-200">
            Remove
          </button>
        </div>
      ))}
      <button
        type="button"
        onClick={onAdd}
        className="text-[10px] text-emerald-200">
        + Add {emptyLabel}
      </button>
    </div>
  );
}
