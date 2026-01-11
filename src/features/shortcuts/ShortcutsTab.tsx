import { useEffect, useState } from "react";
import {
  MousePointer2,
  Command,
  Zap,
  Terminal,
  AlertCircle,
} from "lucide-react";
import { Button } from "../../shared/components/Button";
import { useSettingsStore } from "../../store/settings";
import { useToastStore } from "../../store/toast";

export default function ShortcutsTab() {
  const { shortcutsEnabled, shortcuts, setShortcut, resetShortcuts } =
    useSettingsStore();
  const { addToast } = useToastStore();
  const [recordingAction, setRecordingAction] = useState<
    null | "translate" | "popup" | "enhance" | "terminal"
  >(null);

  useEffect(() => {
    if (!recordingAction) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      // Cancel on Escape
      if (event.key === "Escape") {
        setRecordingAction(null);
        addToast("Recording cancelled", "info");
        return;
      }

      event.preventDefault();
      event.stopPropagation();

      const combo = formatShortcut(event);
      if (!combo) {
        // Still recording, waiting for a non-modifier key
        return;
      }

      // Validate that at least one modifier is pressed
      if (
        !event.ctrlKey &&
        !event.altKey &&
        !event.shiftKey &&
        !event.metaKey
      ) {
        addToast(
          "Shortcut must include at least one modifier (Ctrl, Alt, Shift, or Cmd)",
          "warning"
        );
        return;
      }

      // Check for duplicate shortcuts
      const otherShortcuts = Object.entries(shortcuts).filter(
        ([key]) => key !== recordingAction
      );
      const isDuplicate = otherShortcuts.some(([, value]) => value === combo);
      if (isDuplicate) {
        addToast(
          "This shortcut is already assigned to another action",
          "warning"
        );
        return;
      }

      setShortcut(recordingAction, combo);
      setRecordingAction(null);
      addToast(`Shortcut updated to ${combo}`, "success");
    };

    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [recordingAction, setShortcut, shortcuts, addToast]);

  const shortcutItems = [
    {
      key: shortcuts.translate,
      description:
        "Translate highlighted text and replace clipboard automatically.",
      icon: <Zap className="w-4 h-4 text-amber-500" />,
      action: "Quick Translate",
      id: "translate" as const,
    },
    {
      key: shortcuts.popup,
      description: "Show popup UI and replace selection directly in-place.",
      icon: <MousePointer2 className="w-4 h-4 text-blue-500" />,
      action: "Popup Selector",
      id: "popup" as const,
    },
    {
      key: shortcuts.enhance,
      description: "Enhance the English prompt currently in your clipboard.",
      icon: <Command className="w-4 h-4 text-purple-500" />,
      action: "Enhance Clip",
      id: "enhance" as const,
    },
    {
      key: shortcuts.terminal,
      description:
        "Copy selection, translate, and put in clipboard (no auto-paste) for terminal use.",
      icon: <Terminal className="w-4 h-4 text-green-500" />,
      action: "Terminal Translate",
      id: "terminal" as const,
    },
  ];

  if (!shortcutsEnabled) {
    return null;
  }

  return (
    <div className="max-w-4xl mx-auto p-10 space-y-10">
      <div className="space-y-2">
        <h3 className="text-2xl font-bold tracking-tight text-app-text">
          Keyboard Shortcuts
        </h3>
        <p className="text-app-subtext text-sm">
          Master the speed of gadogado with global desktop hotkeys.
        </p>
      </div>

      <div className="grid gap-4">
        {shortcutItems.map((s, i) => (
          <div
            key={i}
            className="flex items-center justify-between p-6 bg-app-card border border-app-border rounded-xl group hover:border-app-accent/50 transition-all shadow-sm">
            <div className="flex items-center gap-4">
              <div className="p-3 bg-app-bg rounded-lg text-app-text border border-app-border">
                {s.icon}
              </div>
              <div className="space-y-1">
                <h4 className="font-bold text-base text-app-text">
                  {s.action}
                </h4>
                <p className="text-xs text-app-subtext max-w-xs">
                  {s.description}
                </p>
              </div>
            </div>
            <div className="flex items-center gap-4">
              <kbd
                className={`px-3 py-1.5 bg-app-bg border rounded-lg text-sm font-mono font-bold shadow-sm ${
                  recordingAction === s.id
                    ? "border-yellow-500 text-yellow-500 animate-pulse"
                    : "border-app-border text-app-accent"
                }`}>
                {recordingAction === s.id ? "Press keys..." : s.key}
              </kbd>
              <Button
                variant="ghost"
                size="sm"
                className={`opacity-100 border text-xs h-8 ${
                  recordingAction === s.id
                    ? "bg-yellow-500/20 border-yellow-500 text-yellow-500"
                    : "bg-app-bg border-app-border"
                }`}
                onClick={() => {
                  if (recordingAction === s.id) {
                    setRecordingAction(null);
                    addToast("Recording cancelled", "info");
                  } else {
                    setRecordingAction(s.id);
                  }
                }}
                disabled={recordingAction !== null && recordingAction !== s.id}>
                {recordingAction === s.id ? "Cancel" : "Record"}
              </Button>
            </div>
          </div>
        ))}
      </div>
      <div className="flex justify-end">
        <Button
          variant="ghost"
          size="sm"
          className="text-xs border border-app-border bg-app-bg"
          onClick={() => {
            resetShortcuts();
            setRecordingAction(null);
            addToast("Shortcuts restored to defaults", "success");
          }}>
          Restore Defaults
        </Button>
      </div>

      {recordingAction && (
        <div className="p-4 bg-yellow-500/10 border border-yellow-500/30 rounded-xl flex items-start gap-3">
          <AlertCircle className="w-5 h-5 text-yellow-500 flex-shrink-0 mt-0.5" />
          <div className="space-y-1">
            <p className="text-sm text-yellow-200 font-medium">
              Recording shortcut...
            </p>
            <p className="text-xs text-yellow-200/70">
              Press a key combination with at least one modifier (Ctrl, Alt,
              Shift, or Cmd). Press Escape to cancel.
            </p>
          </div>
        </div>
      )}

      <div className="p-6 bg-app-accent/10 border border-app-accent/20 rounded-xl flex items-start gap-4">
        <div className="p-2 bg-app-accent text-white rounded-lg">
          <Zap className="w-4 h-4" />
        </div>
        <div className="space-y-2">
          <h4 className="font-bold text-sm text-app-text">
            Pro Tip: Global Access
          </h4>
          <p className="text-xs text-app-subtext leading-relaxed">
            These shortcuts work even when gadogado is minimized or in the
            background. Use them to translate text directly in your browser or
            text editor.
          </p>
        </div>
      </div>
    </div>
  );
}

function formatShortcut(event: KeyboardEvent) {
  // If only modifier keys are pressed, return null (still recording)
  const modifierKeys = ["Control", "Shift", "Alt", "Meta"];
  if (modifierKeys.includes(event.key)) {
    return null;
  }

  const parts = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Cmd");

  let key = event.key;

  // Normalize special keys
  if (key === " ") key = "Space";
  else if (key === "ArrowUp") key = "ArrowUp";
  else if (key === "ArrowDown") key = "ArrowDown";
  else if (key === "ArrowLeft") key = "ArrowLeft";
  else if (key === "ArrowRight") key = "ArrowRight";
  else if (key.length === 1) {
    // Single character - uppercase it
    key = key.toUpperCase();
  } else {
    // Other special keys - capitalize first letter
    key = key.charAt(0).toUpperCase() + key.slice(1);
  }

  parts.push(key);

  return parts.join(" + ");
}
