import { useState, useEffect } from "react";
import { Check, X, Info, AlertTriangle } from "lucide-react";
import { cn } from "../../utils/cn";

export type ToastType = "success" | "error" | "info" | "warning";

interface ToastProps {
  message: string;
  type?: ToastType;
  duration?: number;
  onClose: () => void;
}

const icons = {
  success: Check,
  error: X,
  info: Info,
  warning: AlertTriangle,
};

const styles = {
  success: "bg-[#1d3326] border-green-600/50 text-green-400",
  error: "bg-[#2a1d1d] border-red-600/50 text-red-400",
  info: "bg-[#1e293b] border-blue-600/50 text-blue-400",
  warning: "bg-[#332e18] border-yellow-600/50 text-yellow-400",
};

export function Toast({
  message,
  type = "success",
  duration = 3000,
  onClose,
}: ToastProps) {
  const [isVisible, setIsVisible] = useState(true);
  const Icon = icons[type];

  useEffect(() => {
    const timer = setTimeout(() => {
      setIsVisible(false);
      setTimeout(onClose, 300);
    }, duration);

    return () => clearTimeout(timer);
  }, [duration, onClose]);

  return (
    <div
      className={cn(
        "flex items-center gap-2 px-4 py-3 rounded-lg border shadow-lg transition-all duration-300",
        styles[type],
        isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-2"
      )}>
      <Icon className="w-4 h-4 flex-shrink-0" />
      <span className="text-sm font-medium">{message}</span>
      <button
        onClick={() => {
          setIsVisible(false);
          setTimeout(onClose, 300);
        }}
        className="ml-2 opacity-60 hover:opacity-100 transition-opacity">
        <X className="w-3 h-3" />
      </button>
    </div>
  );
}
