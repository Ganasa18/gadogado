import { useEffect, useState } from "react";
import { Loader2, X } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isTauri } from "../../utils/tauri";

export default function GlobalLoader() {
  const [message, setMessage] = useState("Processing...");

  useEffect(() => {
    if (!isTauri()) return;
    const unlisten = listen<string>("loading-update", (event) => {
      setMessage(
        event.payload === "translate" ? "Translating..." : "Enhancing..."
      );
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return (
    <div className="relative h-screen w-screen overflow-hidden rounded-xl border border-app-border bg-black/70 text-app-text">
      <div className="absolute right-3 top-3 z-10">
        <button
          className="flex h-7 w-7 items-center justify-center rounded-full border border-white/10 bg-white/5 text-app-subtext transition hover:border-white/30 hover:text-app-text"
          onClick={() => {
            if (!isTauri()) return;
            getCurrentWindow().close().catch(console.error);
          }}
          aria-label="Close loading window">
          <X className="h-3.5 w-3.5" />
        </button>
      </div>
      <div className="relative flex h-full w-full items-center justify-center">
        <div className="flex flex-col items-center gap-3 px-6">
          <Loader2 className="h-10 w-10 text-app-accent animate-spin" />
          <span className="text-sm font-semibold tracking-wide">{message}</span>
        </div>
      </div>
    </div>
  );
}
