import type { ApiFormRow } from "../../hooks/useApiRequestBuilder";
import { createFormRow } from "./rowFactories";

type ApiBodyTabProps = {
  supportsBody: boolean;
  bodyTab: "json" | "form";
  bodyJson: string;
  formData: ApiFormRow[];
  onTabChange: (tab: "json" | "form") => void;
  onBodyJsonChange: (value: string) => void;
  onFormDataChange: (updater: (rows: ApiFormRow[]) => ApiFormRow[]) => void;
};

export function ApiBodyTab({
  supportsBody,
  bodyTab,
  bodyJson,
  formData,
  onTabChange,
  onBodyJsonChange,
  onFormDataChange,
}: ApiBodyTabProps) {
  return (
    <div className="space-y-3">
      {!supportsBody && (
        <div className="text-[11px] text-app-subtext">
          Selected method does not support a request body.
        </div>
      )}
      {supportsBody && (
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => onTabChange("json")}
            className={`px-3 py-1 rounded border text-[10px] transition ${
              bodyTab === "json"
                ? "bg-blue-700/30 border-blue-500/60 text-blue-100"
                : "bg-[#181818] border-app-border text-app-subtext"
            }`}>
            Raw JSON
          </button>
          <button
            type="button"
            onClick={() => onTabChange("form")}
            className={`px-3 py-1 rounded border text-[10px] transition ${
              bodyTab === "form"
                ? "bg-blue-700/30 border-blue-500/60 text-blue-100"
                : "bg-[#181818] border-app-border text-app-subtext"
            }`}>
            Form-data
          </button>
        </div>
      )}
      {supportsBody && bodyTab === "json" && (
        <textarea
          className="w-full min-h-[160px] bg-[#181818] border border-app-border rounded p-2 text-xs outline-none focus:border-gray-500 transition resize-y font-mono"
          placeholder='{"name":"Ada"}'
          value={bodyJson}
          onChange={(event) => onBodyJsonChange(event.target.value)}
        />
      )}
      {supportsBody && bodyTab === "form" && (
        <div className="space-y-2">
          {formData.map((field) => (
            <div
              key={field.id}
              className="grid grid-cols-[auto_1fr_1fr_1fr_auto] gap-2 items-center">
              <input
                type="checkbox"
                checked={field.enabled}
                onChange={(event) =>
                  onFormDataChange((rows) =>
                    rows.map((row) =>
                      row.id === field.id
                        ? { ...row, enabled: event.target.checked }
                        : row,
                    ),
                  )
                }
              />
              <input
                className="bg-[#181818] border border-app-border rounded p-2 text-xs"
                placeholder="key"
                value={field.key}
                onChange={(event) =>
                  onFormDataChange((rows) =>
                    rows.map((row) =>
                      row.id === field.id
                        ? { ...row, key: event.target.value }
                        : row,
                    ),
                  )
                }
              />
              <input
                className="bg-[#181818] border border-app-border rounded p-2 text-xs"
                placeholder="value"
                value={field.value}
                onChange={(event) =>
                  onFormDataChange((rows) =>
                    rows.map((row) =>
                      row.id === field.id
                        ? { ...row, value: event.target.value }
                        : row,
                    ),
                  )
                }
              />
              <input
                type="file"
                onChange={(event) =>
                  onFormDataChange((rows) =>
                    rows.map((row) =>
                      row.id === field.id
                        ? { ...row, file: event.target.files?.[0] ?? null }
                        : row,
                    ),
                  )
                }
              />
              <button
                type="button"
                onClick={() =>
                  onFormDataChange((rows) =>
                    rows.length > 1
                      ? rows.filter((row) => row.id !== field.id)
                      : rows,
                  )
                }
                className="text-[10px] text-red-200">
                Remove
              </button>
            </div>
          ))}
          <button
            type="button"
            onClick={() => onFormDataChange((rows) => [...rows, createFormRow()])}
            className="text-[10px] text-emerald-200">
            + Add field
          </button>
        </div>
      )}
    </div>
  );
}
