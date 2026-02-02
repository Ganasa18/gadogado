import type { ApiResponsePayload } from "../../hooks/useApiRequestBuilder";

type ApiResponsePanelProps = {
  response: ApiResponsePayload | null;
  responseError: string | null;
  formattedBody: string;
};

export function ApiResponsePanel({
  response,
  responseError,
  formattedBody,
}: ApiResponsePanelProps) {
  return (
    <div className="space-y-3">
      {responseError && (
        <div className="text-[11px] text-red-200">{responseError}</div>
      )}
      {!response && !responseError && (
        <div className="text-[11px] text-app-subtext">
          Send a request to view the response.
        </div>
      )}
      {response && (
        <>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-2 text-[11px]">
            <div className="rounded-md border border-app-border bg-black/20 p-2">
              <div className="text-[10px] text-gray-500">Status</div>
              <div className="text-gray-300">{response.status}</div>
            </div>
            <div className="rounded-md border border-app-border bg-black/20 p-2">
              <div className="text-[10px] text-gray-500">Time</div>
              <div className="text-gray-300">{response.durationMs} ms</div>
            </div>
            <div className="rounded-md border border-app-border bg-black/20 p-2">
              <div className="text-[10px] text-gray-500">Content-Type</div>
              <div className="text-gray-300">{response.contentType || "n/a"}</div>
            </div>
          </div>

          <div>
            <div className="text-[10px] text-gray-500 mb-1">Response Headers</div>
            <div className="max-h-[160px] overflow-y-auto space-y-1 text-[10px]">
              {response.headers.length === 0 && (
                <div className="text-app-subtext">No headers available.</div>
              )}
              {response.headers.map((header) => (
                <div key={header.id} className="text-app-subtext">
                  {header.key}: {header.value}
                </div>
              ))}
            </div>
          </div>

          <div>
            <div className="text-[10px] text-gray-500 mb-1">Response Body</div>
            <pre className="bg-black/30 border border-app-border rounded p-3 text-[11px] text-app-text max-h-[320px] overflow-auto whitespace-pre-wrap font-mono">
              {formattedBody || "(empty)"}
            </pre>
          </div>
        </>
      )}
    </div>
  );
}
