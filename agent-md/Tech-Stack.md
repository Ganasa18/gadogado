# PromptBridge: AI-Powered Prompt Translator for Desktop

## Brief Overview

PromptBridge is a lightweight desktop app that helps users translate or enhance Indonesian prompts into English using AI. It supports **local LLMs (via LM Studio)** or **cloud LLM APIs (e.g., OpenAI, Anthropic)**. The app runs fully offline-capable, stores credentials securely, and enables fast workflows via keyboard shortcuts.

---

## Core Features (Only What’s Needed)

- **Bidirectional translation**: From ID To EN | EN
- **Prompt enhancement**: Improve structure, style, and context for LLMs
- **Two LLM modes**:
  - **Local LLM**: Connect to `http://localhost:1234/v1` (LM Studio)
  - **Cloud LLM**: Use API keys (stored securely, never in code)
- **Keyboard shortcuts**:
  - `Ctrl+Alt+T` → Translate selected text and replace clipboard
  - `Ctrl+Alt+P` → Show popup UI and replace selection directly
  - `Ctrl+Alt+E` → Enhance English prompt template
- **Simple UI to manage API keys & LLM settings**
- **Optional local translation history** (user can clear anytime)

> ⚠️ **Boundary**: No analytics, accounts, cloud sync, or push notifications. Only features that directly serve the core workflow.

---

## Core Features

- Terjemahan otomatis prompt (ID ↔ EN)
- Enhance prompt (perbaikan struktur, gaya, dan konteks untuk LLM)
- Dukungan dua mode LLM:
  - **Local LLM**: Menghubungkan ke endpoint LM Studio (misal `http://localhost:1234/v1`)
  - **Cloud LLM**: Menggunakan API Key (disimpan di SQLite terenkripsi)
- Shortcut keyboard:
  - `Ctrl+Alt+T`: Copy selected text → terjemahkan → replace clipboard
  - `Ctrl+Alt+P`: Popup UI → replace selection langsung
  - `Ctrl+Alt+E`: Enhance prompt bahasa Inggris
- Pengelolaan API key & konfigurasi LLM melalui UI
- Riwayat terjemahan lokal (opsional, dapat dihapus)

# Tech Stack

## Frontend (Tauri + Preact)

## **Component** | **Technology** | **Version** | **EOL/Support** |**Docs**

Framework | Preact | ^10.23.0 | Stable | https://preactjs.com/guide/v10/getting-started
Language | TypeScript | ^5.5.0 | Stable| https://www.typescriptlang.org/docs/
Build Tool | Vite | ^5.0.0 | Stable | https://vitejs.dev/
HTTP Client | Axios | ^1.7.0 | Stable | https://axios-http.com/docs/intro
UI Icons | Lucide React | ^0.450.0 | Stable | https://lucide.dev/
Form Handling | React Hook Form | ^7.50.0 | Stable | https://react-hook-form.com/get-started
Validation | Zod | ^3.23.0 | Stable | https://zod.dev/
JSX Support | JSX via Vite + `@preact/preset-vite` | — | Built-in | https://preactjs.com/guide/v10/getting-started#alternatives-to-jsx
React Compatibility Layer | `preact/compat`| ^10.23.0 | Stable | https://preactjs.com/guide/v10/switching-to-preact
Global Shortcuts (Desktop) | `@tauri-apps/plugin-global-shortcut` | ^2.0.0| Stable | https://tauri.app/v1/references/plugins/global-shortcut
Routing | React Router | 7.x| Stable| -
Animation | Frammer Motion | - | -
State (Global) | Zustand | 5.x | Stable | -
State (Server) | Tanstack Query| - | Stable | -
Styling | Tailwind | - | Stable | -

## Backend (Rust)

## **Component** | **Technology** | **Version** | **EOL/Support** | **Docs**

Language | Rust | 1.80+ (MSRV) | No EOL | https://www.rust-lang.org/learn |
Web Framework | Actix Web | ^4.5.0 | Actively maintained | https://actix.rs/docs/ |
Architecture | Clean Layered (Domain / App / Infra) | — | Pattern-based | — |
Database | SQLite | 3.45+ | Public domain, no EOL | https://www.sqlite.org/docs.html |
Database Driver | sqlx | ^0.8.0 | Actively maintained | https://docs.rs/sqlx/latest/sqlx/ |
Secret Management | `keyring` crate | ^2.5.0 | Actively maintained | https://crates.io/crates/keyring |
Configuration | figment + `.env` (`.env` ignored in Git) | ^0.10.0 | Actively maintained | https://github.com/SergioBenitez/Figment |
Logging | `tracing` + `tracing-subscriber` | ^0.1.40 | Actively maintained | https://docs.rs/tracing/latest/tracing/ |
Desktop Integration | Tauri Commands | ^2.0.0 | Actively maintained | https://tauri.app/v1/guides/backend/rust/ |

## Security & Storage

- **API keys**: Stored using OS-native keychain (`keyring`); _no custom AES-256 encryption_
- **No hardcoded secrets** in source or config
- **All inputs validated** with Zod (frontend + backend)
- **Rate limiting**: Applied on LLM requests in backend

## DevOps & Build

# **Purpose** | **Tool** | **Notes**

Build & Packaging | Tauri | Outputs `.exe`, `.dmg`, `.AppImage` |
Auto-update | Tauri Updater (optional) | Disabled by default |
Testing | Vitest (frontend), Playwright (E2E), `cargo test` (Rust) | Minimal but sufficient |

## Non-Functional Requirements

- **Offline-first**: Works without internet if local LLM is available
- **Responsive**: Async LLM calls (no UI freeze)
- **Lightweight**: < 200 MB RAM at idle
- **Private**: All data remains on-device

# Design Layout

- has tab button for General | Shortucst | Tutorial
- General: 2 grid tools function | logging
- First Column Grid Function: Dropdown Free (Local llm) | Charge (Claude)
- Second Colom on tab general Select Model

# Rules

- dont use npm run build
- use existing ui component
- tyspcripty dont use type any
- if need verify search on internet
