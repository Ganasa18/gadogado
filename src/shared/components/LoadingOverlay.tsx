import { useEffect, useState } from "react";
import { Loader2 } from "lucide-react";

interface LoadingOverlayProps {
  forceShow?: boolean;
}

export default function LoadingOverlay({ forceShow }: LoadingOverlayProps) {
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState("");

  useEffect(() => {
    const handleStart = (event: any) => {
      setLoading(true);
      setMessage(
        event.detail === "translate" ? "Translating..." : "Enhancing..."
      );
    };

    const handleEnd = () => {
      setLoading(false);
      setMessage("");
    };

    window.addEventListener("shortcut-start", handleStart);
    window.addEventListener("shortcut-end", handleEnd);

    return () => {
      window.removeEventListener("shortcut-start", handleStart);
      window.removeEventListener("shortcut-end", handleEnd);
    };
  }, []);

  const isVisible = forceShow || loading;

  if (!isVisible) return null;

  return (
    <div className="fixed inset-0 z-[9999] flex items-center justify-center bg-black/60 backdrop-blur-md animate-in fade-in duration-300">
      <div className="bg-app-panel border border-app-border rounded-2xl shadow-2xl p-8 flex flex-col items-center gap-6 min-w-[240px] animate-in zoom-in-95 duration-300">
        <div className="relative">
          <Loader2 className="w-12 h-12 text-app-accent animate-spin" />
          <div className="absolute inset-0 flex items-center justify-center">
            <div className="w-6 h-6 bg-app-accent/20 rounded-full blur-xl animate-pulse"></div>
          </div>
        </div>
        <div className="flex flex-col items-center gap-1">
          <span className="text-app-text font-bold text-base tracking-tight">
            {message || "Processing Request"}
          </span>
          <span className="text-app-subtext text-[11px] uppercase tracking-widest font-medium opacity-60">
            Please wait a moment
          </span>
        </div>
      </div>
    </div>
  );
}
