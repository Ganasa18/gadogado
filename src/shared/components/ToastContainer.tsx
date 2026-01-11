import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { useToastStore } from "../../store/toast";
import { Toast } from "./Toast";

export function ToastContainer() {
  const { toasts, removeToast } = useToastStore();
  const [fullscreenElement, setFullscreenElement] =
    useState<HTMLElement | null>(
      typeof document !== "undefined"
        ? (document.fullscreenElement as HTMLElement | null)
        : null
    );

  useEffect(() => {
    const handleFullscreenChange = () => {
      setFullscreenElement(
        typeof document !== "undefined"
          ? (document.fullscreenElement as HTMLElement | null)
          : null
      );
    };
    document.addEventListener("fullscreenchange", handleFullscreenChange);
    return () => {
      document.removeEventListener("fullscreenchange", handleFullscreenChange);
    };
  }, []);

  if (toasts.length === 0) return null;

  const positionClasses = fullscreenElement
    ? "absolute bottom-4 right-4"
    : "fixed bottom-4 right-4";

  const content = (
    <div
      className={`${positionClasses} z-50 flex flex-col gap-2`}
      data-qa-record-ignore>
      {toasts.map((toast) => (
        <Toast
          key={toast.id}
          message={toast.message}
          type={toast.type}
          duration={toast.duration}
          onClose={() => removeToast(toast.id)}
        />
      ))}
    </div>
  );

  if (fullscreenElement) {
    return createPortal(content, fullscreenElement);
  }

  return content;
}
