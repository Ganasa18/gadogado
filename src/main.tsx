import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClientProvider } from "@tanstack/react-query";
import App from "./app/App";
import { queryClient } from "./app/queryClient";
import GlobalLoader from "./shared/components/GlobalLoader";
const isLoader =
  window.location.search.includes("label=loading") || window.name === "loading";
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      {isLoader ? <GlobalLoader /> : <App />}
    </QueryClientProvider>
  </React.StrictMode>
);
