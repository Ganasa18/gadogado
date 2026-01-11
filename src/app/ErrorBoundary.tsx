import { useRouteError, isRouteErrorResponse } from "react-router";
import { AlertTriangle } from "lucide-react";

export default function ErrorBoundary() {
  const error = useRouteError();

  let errorMessage: string;
  let errorStatus: number | undefined;

  if (isRouteErrorResponse(error)) {
    errorStatus = error.status;
    errorMessage = error.statusText || error.data?.message || "An error occurred";
  } else if (error instanceof Error) {
    errorMessage = error.message;
  } else {
    errorMessage = "An unknown error occurred";
  }

  return (
    <div className="flex items-center justify-center h-screen bg-app-bg text-app-text">
      <div className="max-w-md w-full mx-4">
        <div className="bg-app-card border border-app-border rounded-lg p-8 shadow-lg">
          <div className="flex items-center gap-3 mb-4">
            <AlertTriangle className="w-8 h-8 text-red-500" />
            <h1 className="text-2xl font-bold">
              {errorStatus ? `Error ${errorStatus}` : "Oops!"}
            </h1>
          </div>
          <p className="text-app-subtext mb-6">{errorMessage}</p>
          <button
            type="button"
            onClick={() => window.location.reload()}
            className="w-full bg-app-accent hover:bg-app-accent/80 text-white font-medium py-2 px-4 rounded-md transition-colors">
            Reload Application
          </button>
        </div>
      </div>
    </div>
  );
}
