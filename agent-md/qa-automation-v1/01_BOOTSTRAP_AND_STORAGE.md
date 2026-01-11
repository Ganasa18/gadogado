# FEATURE 01 — Bootstrap Project & Storage

## Objective

Set up the Tauri project foundation and SQLite storage.

## Requirements

- Tauri backend (Rust)
- Local SQLite database
- Per-session data folders

## Tasks

Status legend:

- [ ] Not started
- [~] In progress
- [x] Done

Checklist:

- [x] Create a helper to resolve the app data directory
- [x] Create folder structure: app_data/qa_sessions/<session_id>/
- [x] Initialize SQLite on app startup
- [x] Auto-create tables if they do not exist

## Rules

- Do NOT store binary files in SQLite
- Store file paths as TEXT only

## Acceptance Criteria

- App starts without errors
- Session folders can be created
- SQLite is ready for other features

### Tests

- Command: `cargo check`

## Exit Criteria (Tests)

- [~] Run Feature 01 tests (record command or checklist used)
  - [x] Run log: Initialized QA database at: C:\Users\YSHA-PC\AppData\Roaming\com.ysha-pc.gadogado\qa_recorder.db
  - [x] Run log: QA database file created: C:\Users\YSHA-PC\AppData\Roaming\com.ysha-pc.gadogado\qa_recorder.db (110592 bytes)
  - [x] Run log: QA database ready, proceeding to app database init
  - [x] Run log: QA session directory created successfully (bootstrap session)
- [~] All Feature tests pass
  - [x] Run created seassion

Status: In progress (exit criteria checks pending).
