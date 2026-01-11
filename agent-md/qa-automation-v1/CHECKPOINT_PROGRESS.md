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
- [ ] 05 Checkpoint System
- [ ] 06 Event Chunking & Summary
- [ ] 07 AI Test Generation
- [ ] 08 Deduplication & Priority
- [ ] 09 Report Export

## Global References (Task Progress)

- [ ] 00_PROJECT_OVERVIEW.md tracking rules confirmed
- [x] 00A_DATABASE_SETUP.md tasks complete
- [x] 00B_RUST_LIBRARIES.md tasks complete

## Feature 01 Tasks

- [x] Create a helper to resolve the app data directory
- [x] Create folder structure: app_data/qa_sessions/<session_id>/
- [x] Initialize SQLite on app startup
- [x] Auto-create tables if they do not exist

## Feature 02 Tasks

- [x] Create session
- [x] Manual start/stop verified in app
- [x] List session

## Feature 03 Tasks

- [x] Create event
- [x] Get events by session id
- [x] Record events successfully (log + UI feedback)
- [x] Delete selected events (keep unselected)
- [x] Activity stream pagination

## Feature 04 Tasks

- [x] Store screenshot and link to event
- [ ] Add checkpoint creation on recording success

## Note

- **Backend logging**: Ensure new backend requests add `add_log()` calls without logging sensitive data.
- **Library check**: No new Rust libraries added for Event Recorder (manual verification pending).
- **Checkpoint**: Screenshot capture stores files and links events; checkpoint creation still pending.
- **Blockers**: None.
- **Next feature**: `05 Checkpoint System`
