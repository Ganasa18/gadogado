import { useEffect } from "react";
import { useThemeStore, type AppMode } from "../themeStore";

export default function ThemeManager() {
  const { theme, mode } = useThemeStore();

  useEffect(() => {
    const root = window.document.documentElement;

    // Apply Theme Attribute
    root.setAttribute("data-theme", theme);

    // Handle Mode
    const applyMode = (targetMode: AppMode) => {
      const isDark =
        targetMode === "dark" ||
        (targetMode === "system" &&
          window.matchMedia("(prefers-color-scheme: dark)").matches);

      if (isDark) {
        root.classList.add("dark");
      } else {
        root.classList.remove("dark");
      }
    };

    applyMode(mode);

    // Listen for system changes if in system mode
    if (mode === "system") {
      const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
      const handleChange = () => applyMode("system");
      mediaQuery.addEventListener("change", handleChange);
      return () => mediaQuery.removeEventListener("change", handleChange);
    }
  }, [theme, mode]);

  return null;
}
