import { useCallback, useEffect, useMemo, useState } from "react";
import { Activity, ChevronRight, Settings } from "lucide-react";
import { useShallow } from "zustand/shallow";
import { useSettingsStore } from "./store/settings";
import TranslateTab from "./features/translate/TranslateTab";
import EnhanceTab from "./features/enhance/EnhanceTab";
import TypeGenTab from "./features/typegen/TypeGenTab";
import GeneralTab from "./features/settings/GeneralTab";
import HistoryTab from "./features/history/HistoryTab";
import ShortcutsTab from "./features/shortcuts/ShortcutsTab";
import TutorialTab from "./features/tutorial/TutorialTab";
import FeedbackTab from "./features/feedback/FeedbackTab";
import TokenTab from "./features/token/TokenTab";
import SessionManagerTab from "./features/qa/SessionManagerTab";
import PopupTranslateDialog from "./shared/components/PopupTranslateDialog";
import { ToastContainer } from "./shared/components/ToastContainer";
import TerminalPanel from "./shared/components/TerminalPanel";
import { NAV_SECTIONS, type NavItem } from "./app/navigation";
import { cn } from "./utils/cn";
import { useShortcutEvents } from "./hooks/useShortcutEvents";
import { useSyncConfig } from "./hooks/useSyncConfig";
import { useSyncLanguages } from "./hooks/useSyncLanguages";
import { useSyncShortcuts } from "./hooks/useSyncShortcuts";
import "./App.css";

const DEFAULT_PATH = "/general";

function App() {
  const [capturedText, setCapturedText] = useState<string | null>(null);
  const [popupOpen, setPopupOpen] = useState(false);
  const { provider, model, shortcutsEnabled } = useSettingsStore(
    useShallow((state) => ({
      provider: state.provider,
      model: state.model,
      shortcutsEnabled: state.shortcutsEnabled,
    }))
  );

  useSyncConfig();
  useSyncLanguages();
  useSyncShortcuts();

  const handleCapture = useCallback((payload: string) => {
    setCapturedText(payload);
    setPopupOpen(true);
  }, []);

  useShortcutEvents({ onCapture: handleCapture });

  const visibleSections = useMemo(
    () =>
      NAV_SECTIONS.map((section) => ({
        ...section,
        items: section.items.filter(
          (item) => !item.requiresShortcuts || shortcutsEnabled
        ),
      })),
    [shortcutsEnabled]
  );

  const visibleItems = useMemo(
    () => visibleSections.flatMap((section) => section.items),
    [visibleSections]
  );

  const [activePath, setActivePath] = useState<string>(DEFAULT_PATH);

  useEffect(() => {
    if (!visibleItems.some((item) => item.path === activePath)) {
      setActivePath(DEFAULT_PATH);
    }
  }, [activePath, visibleItems]);

  const activeItem = useMemo(
    () => visibleItems.find((item) => item.path === activePath),
    [visibleItems, activePath]
  );

  const ActiveIcon = activeItem?.icon ?? Settings;
  const activeSection = activeItem?.section ?? "Configuration";
  const activeLabel = activeItem?.label ?? "General";

  const clearCapturedText = useCallback(() => setCapturedText(null), []);
  const handleNavigate = useCallback((path: string) => {
    setActivePath(path);
  }, []);

  const activeContent = useMemo(() => {
    switch (activePath) {
      case "/translate":
        return (
          <TranslateTab
            initialText={capturedText}
            onTextConsumed={clearCapturedText}
          />
        );
      case "/enhance":
        return <EnhanceTab />;
      case "/typegen":
        return <TypeGenTab />;
      case "/history":
        return <HistoryTab />;
      case "/qa":
        return <SessionManagerTab />;
      case "/token":
        return <TokenTab />;
      case "/shortcut":
        return shortcutsEnabled ? <ShortcutsTab /> : <GeneralTab />;
      case "/feedback":
        return <FeedbackTab />;
      case "/tutorial":
        return <TutorialTab />;
      case "/general":
      default:
        return <GeneralTab />;
    }
  }, [activePath, capturedText, clearCapturedText, shortcutsEnabled]);

  return (
    <main className="flex h-screen w-screen overflow-hidden bg-app-bg text-app-text select-none font-sans flex-col">
      <header
        className="flex-none bg-app-bg border-b border-black/20"
        data-purpose="title-bar">
        <div className="flex justify-between items-center px-4 py-2">
          <div className="flex items-center gap-2">
            <Activity className="w-4 h-4 text-app-success" />
            <span className="font-semibold tracking-wide text-sm">
              gadogado
            </span>
          </div>
        </div>
        <div className="px-4 py-3 border-b border-app-border flex items-center gap-2 text-app-subtext text-xs">
          <ActiveIcon className="w-3.5 h-3.5" />
          <span>{activeSection}</span>
          <span className="text-gray-600">/</span>
          <span className="text-white font-medium capitalize">
            {activeLabel}
          </span>
        </div>
      </header>

      <div className="flex-1 flex overflow-hidden">
        <nav className="flex flex-col w-64 border-r border-app-border bg-app-panel">
          <div className="flex-1 overflow-y-auto py-4 px-4 space-y-6">
            {visibleSections.map((section) => (
              <div key={section.id} className="space-y-1.5">
                <div className="px-2 pb-2 text-[10px] uppercase font-bold text-app-subtext/60 tracking-widest">
                  {section.id}
                </div>
                {section.items.map((item) => (
                  <NavItem
                    key={item.path}
                    item={item}
                    isActive={item.path === activePath}
                    onNavigate={handleNavigate}
                  />
                ))}
              </div>
            ))}
          </div>

          <div className="p-4 mt-auto border-t border-app-border bg-black/10">
            <div className="flex flex-col gap-2">
              <div className="flex items-center justify-between text-[10px]">
                <span className="text-app-subtext font-medium uppercase tracking-wider">
                  Active Mode
                </span>
                <span
                  className={`font-bold ${
                    provider === "local" ||
                    provider === "ollama" ||
                    provider === "llama_cpp"
                      ? "text-app-success"
                      : "text-app-accent"
                  }`}>
                  {provider === "local"
                    ? "Local LLM"
                    : provider === "ollama"
                    ? "Ollama"
                    : provider === "llama_cpp"
                    ? "Llama.cpp"
                    : provider === "openai"
                    ? "OpenAI"
                    : provider === "gemini"
                    ? "Gemini"
                    : "DLL"}

                </span>
              </div>
              <div className="flex items-center justify-between text-[10px]">
                <span className="text-app-subtext font-medium uppercase tracking-wider">
                  Model ID
                </span>
                <span className="truncate  text-app-text font-bold">
                  {model}
                </span>
              </div>
            </div>
          </div>
        </nav>

        <section className="flex-1 flex flex-col min-w-0 bg-app-bg relative overflow-hidden">
          <div className="flex-1 overflow-y-auto relative no-scrollbar">
            {activeContent}
          </div>
        </section>
      </div>
      <TerminalPanel />
      <PopupTranslateDialog
        open={popupOpen}
        initialText={capturedText}
        onClose={() => {
          setPopupOpen(false);
          setCapturedText(null);
        }}
      />
      <ToastContainer />
    </main>
  );
}

function NavItem({
  item,
  isActive,
  onNavigate,
}: {
  item: NavItem;
  isActive: boolean;
  onNavigate: (path: string) => void;
}) {
  const Icon = item.icon;

  return (
    <button
      type="button"
      onClick={() => onNavigate(item.path)}
      aria-current={isActive ? "page" : undefined}
      className={cn(
        "flex w-full justify-start gap-3 h-9 px-3 transition-all duration-200 rounded-md border border-transparent items-center bg-transparent",
        isActive
          ? "bg-app-card text-app-text border-app-border shadow-sm"
          : "text-app-subtext hover:text-app-text hover:bg-app-card/50"
      )}>
      <div className={isActive ? "text-app-accent" : "text-app-subtext"}>
        <Icon className="w-4 h-4" />
      </div>
      <span
        className={cn(
          "text-[13px] font-medium tracking-tight",
          isActive && "text-app-text"
        )}>
        {item.label}
      </span>
      {isActive && (
        <ChevronRight className="ml-auto w-3.5 h-3.5 text-app-subtext/40" />
      )}
    </button>
  );
}

export default App;
