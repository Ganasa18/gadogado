import { useCallback, useEffect, useMemo, useState } from "react";
import { Outlet, useLocation, useNavigate } from "react-router";
import { Activity, ChevronRight, Settings } from "lucide-react";
import { useShallow } from "zustand/shallow";
import { useSettingsStore } from "../store/settings";
import PopupTranslateDialog from "../shared/components/PopupTranslateDialog";
import { ToastContainer } from "../shared/components/ToastContainer";
import TerminalPanel from "../shared/components/TerminalPanel";
import { NAV_SECTIONS, type NavItem } from "./navigation";
import { cn } from "../utils/cn";
import { useShortcutEvents } from "../hooks/useShortcutEvents";
import { useQaEventRecorder } from "../hooks/useQaEventRecorder";
import { useSyncConfig } from "../hooks/useSyncConfig";
import { useSyncLanguages } from "../hooks/useSyncLanguages";
import { useSyncShortcuts } from "../hooks/useSyncShortcuts";
import "./App.css";

export default function Layout() {
  const location = useLocation();
  const navigate = useNavigate();
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
  useQaEventRecorder();

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

  const rawPath = location.pathname === "/" ? "/general" : location.pathname;
  const activePath = rawPath.startsWith("/qa/session/")
    ? "/qa/history"
    : rawPath;

  useEffect(() => {
    if (
      location.pathname === "/" ||
      !visibleItems.some((item) => item.path === activePath)
    ) {
      navigate("/general", { replace: true });
    }
  }, [location.pathname, visibleItems, activePath, navigate]);

  const activeItem = useMemo(
    () => visibleItems.find((item) => item.path === activePath),
    [visibleItems, activePath]
  );

  const ActiveIcon = activeItem?.icon ?? Settings;
  const activeSection = activeItem?.section ?? "Configuration";
  const activeLabel = activeItem?.label ?? "General";

  const handleNavigate = useCallback(
    (path: string) => {
      navigate(path);
    },
    [navigate]
  );

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
                  <NavItemButton
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
                    provider === "local"
                      ? "text-app-success"
                      : "text-app-accent"
                  }`}>
                  {provider === "local"
                    ? "Local LLM"
                    : provider === "openai"
                    ? "Open Api"
                    : provider === "google"
                    ? "Google"
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
            <Outlet context={{ capturedText, onTextConsumed: () => setCapturedText(null) }} />
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

function NavItemButton({
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
