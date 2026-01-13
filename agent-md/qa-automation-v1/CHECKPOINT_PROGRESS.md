# QA Recorder — Development Progress

## Status Legend

- [ ] Not started
- [~] In progress
- [x] Done

## Definition of "Done"

A feature is marked [x] only when:

- All tasks are implemented AND
- The functionality is integrated into the app startup or relevant workflow AND
- Behavior matches the acceptance criteria in the corresponding .md file (e.g., 01_BOOTSTRAP_AND_STORAGE.md) AND
- Validated via `cargo check` + manual/runtime verification (no unit tests required).
- Every add request front end to backend on backend call function add_log() [for log on frontend log terminal]

## Core Features

- [~] 01 Bootstrap & Storage
- [x] 02 Session Manager
- [~] 03 Event Recorder
- [x] 04 Screenshot Capture
- [~] 05 Checkpoint System
- [~] 06 Event Chunking & Summary
- [~] 07 AI Test Generation

- [~] 08 Deduplication & Priority

- [ ] 09 Report Export

## Global References (Task Progress)

- [ ] 00_PROJECT_OVERVIEW.md tracking rules confirmed
- [x] 00A_DATABASE_SETUP.md tasks complete
- [x] 00B_RUST_LIBRARIES.md tasks complete

## Schema Workflow Updates

- [x] Session/run modeling tables added
- [x] Event + API logging captured in schema
- [~] AI-triggered exploration & test generation (current focus)
- [ ] Streaming integration (WebSocket progress/events)
- [ ] UI binding for run/test results
- [ ] Validation queries for run/event/api coverage

## Feature 01 Tasks


- [x] Create a helper to resolve the app data directory
- [x] Create folder structure: app_data/qa_sessions/<session_id>/
- [x] Initialize SQLite on app startup
- [x] Auto-create tables if they do not exist

## Feature 02 Tasks

- [x] Create session
- [x] Manual start/stop verified in app
- [x] List session
- [x] Add session type metadata (browser/api)


## Feature 03 Tasks

- [x] Create event
- [x] Get events by session id
- [x] Record events successfully (log + UI feedback)
- [~] Replay recorded event from timeline (implementation ready, needs manual verification)
- [~] Replay full session flow (browser/api) (implementation ready, needs manual verification)
- [x] Delete selected events (keep unselected)
- [x] Activity stream pagination


## Feature 04 Tasks
 
- [x] Store screenshot and link to event
- [x] Harden native capture bounds + capture modes
- [~] Add checkpoint creation on recording success
 
## Note


- **Backend logging**: Ensure new backend requests add `add_log()` calls without logging sensitive data.
- **Library check**: No new Rust libraries added for Event Recorder (manual verification pending).
- **Checkpoint**: Auto/manual checkpoint creation wired; requires post-submit result observed before creation; needs runtime verification.
- **Screenshot capture**: Added window/full capture modes with bounds clamping; manual verification pending.
- **Replay**: Browser replay runs deterministic event queue with iframe acknowledgements; API replay executes requests in order (cURL modal fallback in web).
- **AI outputs**: Summary/test generation and AI results view added; require post-submit result observed before generation; needs runtime verification.
- **Current focus**: AI-triggered exploration & test generation workflow.
- **Next steps**: Streamed run progress integration, UI binding for results, validation queries.
- **Blockers**: None.
- **Next feature**: `08 Deduplication & Priority` (in progress)
