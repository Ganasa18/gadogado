# Folder Structure Guide - gadogado [READONLY]

## Goals

- Make the codebase easy to navigate as features grow
- Separate concerns by layer and by feature
- Keep shared code explicit and stable
- On every backend request add logging to tracing error with function add_log() [don't log sensitive data]

repo/
├── agent-md/ # Project documentation & QA automation specs
│ ├── Architecture.md # High-level system architecture
│ ├── Backend.md # Backend design principles
│ ├── Frontend.md # Frontend architecture guide
│ ├── Security.md # Security model (encryption, keychain, etc.)
│ ├── Folder-Structure.md # Overview of full project layout
│ ├── Tech-Stack.md # Approved libraries & tools
│ └── qa-automation-v1/ # QA Recorder MVP documentation
│ ├── 00_PROJECT_OVERVIEW.md
│ ├── 00A_DATABASE_SETUP.md
│ ├── 00B_RUST_LIBRARIES.md
│ ├── 01_BOOTSTRAP_AND_STORAGE.md
│ ├── 02_SESSION_MANAGER.md
│ ├── 03_EVENT_RECORDER.md
│ ├── 04_SCREENSHOT_CAPTURE.md
│ ├── 05_CHECKPOINT_SYSTEM.md
│ ├── 06_EVENT_CHUNKING_AND_SUMMARY.md
│ ├── 07_AI_TEST_GENERATION.md
│ ├── 08_DEDUP_AND_PRIORITY.md
│ ├── 09_REPORT_EXPORT.md
│ ├── 10_FEATURE_VERIFICATION_CHECKLIST.md
│ ├── CHECKPOINT_PROGRESS.md
│ └── STRUCTURE_DOCS.md # Links to src/ and src-tauri/ structure guides
│
├── public/ # Static assets served directly (favicons, manifest, etc.)
├── src/ # Frontend source (Preact + Vite)
├── src-tauri/ # Tauri backend (Rust)
├── package.json
└── vite.config.ts

## Frontend Structure (src/)

src/
├── app/ # App shell: root layout, routing, error handling
│ ├── App.tsx # Root component with RouterProvider
│ ├── router.tsx # Route config using createBrowserRouter
│ ├── Layout.tsx # Root layout with <Outlet />
│ └── ErrorBoundary.tsx # Global error boundary
│
├── features/ # Feature-first modules (co-located logic)
│ ├── translate/
│ │ ├── components/ # Feature-specific UI
│ │ ├── hooks/ # Custom hooks (e.g., useTranslation)
│ │ ├── api/ # Feature API clients
│ │ ├── types.ts # Feature-specific types
│ │ └── index.ts # Public API (optional re-exports)
│ │
│ ├── enhance/
│ ├── history/
│ ├── settings/
│ ├── shortcuts/
│ ├── qa/
│ ├── token/
│ ├── tutorial/
│ └── feedback/
│
├── shared/ # Truly reusable cross-feature code
│ ├── components/ # Generic UI (Button, Modal, Card, etc.)
│ └── api/ # Base HTTP client, interceptors, error handling
│
├── hooks/ # App-wide custom hooks (e.g., useDebounce, useLocalStorage)
├── store/ # Zustand global state stores
├── utils/ # Pure utility functions (formatDate, uuid, etc.)
├── assets/ # Static files: images, icons, fonts
├── types/ # Global TypeScript interfaces/enums
└── api/ # Re-exports from shared/api for cleaner imports

Guidelines:

- Keep feature-specific logic inside `features/`.
- Use `components/` only for reusable UI pieces.
- Split large screens into subcomponents and co-locate them.

## Routing with React Router

### Route Configuration Pattern

Use `createBrowserRouter` for route configuration with support for loaders, actions, and nested routes:

```tsx
// app/router.tsx
import { createBrowserRouter } from "react-router";
import Layout from "./Layout";
import ErrorBoundary from "./ErrorBoundary";

export const router = createBrowserRouter([
  {
    path: "/",
    element: <Layout />,
    errorElement: <ErrorBoundary />,
    children: [
      {
        index: true,
        element: <HomePage />,
      },
      {
        path: "translate",
        element: <TranslatePage />,
      },
      {
        path: "settings",
        element: <SettingsPage />,
      },
    ],
  },
]);
```

### Layout Component Pattern

Use `<Outlet />` to render nested routes:

```tsx
// app/Layout.tsx
import { Outlet } from "react-router";

export default function Layout() {
  return (
    <div>
      <nav>{/* navigation */}</nav>
      <main>
        <Outlet />
      </main>
    </div>
  );
}
```

### Navigation Patterns

Use `Link` for declarative navigation and `useNavigate` for programmatic navigation:

```tsx
import { Link, useNavigate } from "react-router";

function MyComponent() {
  const navigate = useNavigate();

  const handleAction = () => {
    navigate("/settings");
  };

  return (
    <div>
      <Link to="/translate">Go to Translate</Link>
      <button onClick={handleAction}>Settings</button>
    </div>
  );
}
```

### Feature-Based Routes

Each feature can export its own route configuration:

```tsx
// features/translate/routes.tsx
export const translateRoutes = {
  path: "translate",
  children: [
    { index: true, element: <TranslateMain /> },
    { path: "history", element: <TranslateHistory /> },
  ],
};
```

## QA Automation Docs (agent-md/qa-automation-v1)

Purpose:

- Track the QA Recorder MVP plan, schema, dependencies, and feature progress
- Keep feature docs small and scoped to one milestone

Guidelines:

- Update `CHECKPOINT_PROGRESS.md` whenever a task starts or completes
- Keep each feature file focused on objectives, tasks, and acceptance criteria
- Treat `00*_` files as global references (overview, DB, dependencies)

## Backend Structure (src-tauri/src/)

src-tauri/src/
├── shared/ # Shared types, errors, and utilities (no deps on other local modules)
│ ├── errors.rs # AppError, ErrorKind, From<...> impls
│ ├── types.rs # Common types: SessionId, Timestamp, Result alias
│ └── utils.rs # Cross-cutting helpers (e.g., resolve_app_data_dir)
│
├── domain/ # Pure business logic — no I/O, no async, no external crates
│ ├── entities/ # Core business objects (e.g., Prompt, Translation)
│ ├── value_objects/ # Validated immutable types (e.g., NonEmptyString)
│ └── errors.rs # Domain-specific error variants
│
├── application/ # Use cases orchestrating domain + infrastructure
│ └── use_cases/
│ ├── translate.rs # Translate prompt use case
│ └── enhance.rs # Enhance prompt use case
│
├── infrastructure/ # External concerns — I/O, side effects, frameworks
│ ├── config/ # App config loading (from env or file)
│ ├── response/ # LLM output parsing & sanitization
│ ├── db/
│ │ ├── main/ # For promptbridge.db (user data)
│ │ │ ├── models/
│ │ │ ├── repositories/
│ │ │ └── connection.rs
│ │ │
│ │ └── qa/ # For qa_recorder.db (QA automation only)
│ │ ├── migrations/ # schema.sql or .sql files
│ │ ├── models/ # sqlx::FromRow structs for QA tables
│ │ ├── repositories/ # SessionRepository, EventRepository, etc.
│ │ └── connection.rs # init_qa_db(), initialize_schema()
│ │
│ ├── llm_clients/ # Adapters: OpenAI, Ollama, local LLMs
│ └── security/ # AES-256, OS keychain, credential storage
│
├── interfaces/ # Adapters to external systems
│ ├── tauri/ # Tauri command handlers (thin delegators)
│ │ ├── app_commands.rs # e.g., translate_prompt, save_token
│ │ └── qa_commands.rs # e.g., qa_start_session, qa_end_session
│ │
│ └── http/ # Optional: local debug API (e.g., /debug/logs)
│
├── main.rs # Tauri entry point — minimal setup logic
└── lib.rs # Public API (optional; mainly for integration tests)

Guidelines:

- Keep domain logic free of I/O.
- Keep infrastructure behind interfaces.
- Keep Tauri commands thin and delegate to application layer.

## When to Split Files

- If a file exceeds ~300 lines or mixes multiple concerns
- When a feature has its own types, API calls, and UI
- When tests or mocks are needed for a single component
