import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClientProvider } from "@tanstack/react-query";
import App from "./app/App";
import { queryClient } from "./app/queryClient";
import GlobalLoader from "./shared/components/GlobalLoader";
import DetachedTerminal from "./shared/components/DetachedTerminal";

const isLoader =
  window.location.search.includes("label=loading") || window.name === "loading";
const isTerminalDetach = window.location.search.includes("label=terminal");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      {isTerminalDetach ? (
        <DetachedTerminal />
      ) : isLoader ? (
        <GlobalLoader />
      ) : (
        <App />
      )}
    </QueryClientProvider>
  </React.StrictMode>
);
