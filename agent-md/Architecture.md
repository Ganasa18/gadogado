# System Architecture - gadogado

## High-Level Overview

gadogado is a desktop prompt translation and enhancement app built with:

- Frontend: Tauri + React + React Router (UI, navigation, shortcuts, dialogs)
- Backend: Rust + Actix Web (LLM gateway, config, storage)
- Storage: SQLite (settings, optional history)
- LLM providers: Local (LM Studio) and Cloud (OpenAI, Google, DLL)

Communication uses Tauri IPC for production and optional local HTTP on localhost for debugging.

## Runtime Components

+------------------------+ IPC / local HTTP +-------------------+ +------------------+
| Frontend | <----------------------> | Backend | <-> | LLM Endpoint |
| Tauri + React + Router | | Rust + Actix | | Local or Cloud |
+------------------------+ +-------------------+ +------------------+
| |
+------------------------- SQLite --------------------+

## Data Flow (Translation)

1. User presses the translate shortcut.
2. Frontend reads selected text or clipboard.
3. Frontend invokes a backend command (`translate_prompt`).
4. Backend validates input, loads config, and calls the LLM provider.
5. Backend returns translated text (and optionally logs/history).
6. Frontend updates the clipboard and UI state.

## Window Behavior

- A loading overlay window is shown during global shortcut operations.
- The overlay is always-on-top and centered; the main window is minimized.
- The overlay includes a close button for stalled requests.

## Popup Translate Dialog

- Popup shortcut captures selected text and opens a modal dialog.
- The dialog auto-translates and supports Copy and Regenerate.
- Language selectors are shared with main settings.

## Shortcut Configuration

- Shortcut labels are stored in local settings and editable via UI.
- Backend shortcut registration remains stable; the UI shows user labels.

## Frontend Navigation

The app uses React Router for client-side navigation:

- **Router Configuration**: Centralized route definitions in `app/router.tsx` using `createBrowserRouter`
- **Layout System**: Root layout component with `<Outlet />` for rendering nested routes
- **Navigation Methods**:
  - Declarative: `<Link>` components for navigation links
  - Programmatic: `useNavigate()` hook for navigation after actions
- **Route Organization**: Feature-based routes co-located with feature modules
- **Error Handling**: Global error boundary integrated with routing system

Navigation is hash-based (`HashRouter`) to avoid conflicts with Tauri's file protocol.

## Storage

- Database file: `promptbridge.db`
- Location: OS-specific app data directory
  - Windows: `%APPDATA%\PromptBridge`
  - macOS: `~/Library/Application Support/PromptBridge`
- Encryption: Optional via SQLCipher (build-time)

## Logging and Observability

- Logs are local-only and visible in the in-app terminal.
- No analytics or external telemetry.

## Related Docs

- Backend: `agent-md/Backend.md`
- Frontend: `agent-md/FrontEnd.md`
- Security: `agent-md/Security.md`
- Folder Structure: `agent-md/Folder-Structure.md`
