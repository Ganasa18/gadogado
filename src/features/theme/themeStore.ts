import { create } from "zustand";
import { persist } from "zustand/middleware";

export type AppTheme = "default" | "pastel-blue" | "pastel-green";
export type AppMode = "light" | "dark" | "system";

interface ThemeState {
  theme: AppTheme;
  mode: AppMode;
  setTheme: (theme: AppTheme) => void;
  setMode: (mode: AppMode) => void;
}

export const useThemeStore = create<ThemeState>()(
  persist(
    (set) => ({
      theme: "default",
      mode: "system",
      setTheme: (theme) => set({ theme }),
      setMode: (mode) => set({ mode }),
    }),
    {
      name: "gadogado-theme-storage",
    }
  )
);
