export function isTauri() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
