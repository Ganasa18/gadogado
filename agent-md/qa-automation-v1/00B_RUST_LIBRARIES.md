# RUST LIBRARIES — MVP Stack

## Purpose

Define approved Rust libraries for the MVP to avoid overengineering
and reduce implementation friction when using Codex / Claude.

---

## Tasks (Progress)

- [ ] Confirm required crates and lock versions
- [x] Choose chrono vs std::time for timestamps
- [x] Choose HTTP client (reqwest vs ureq)
- [ ] Decide screenshot crate (scrap vs xcap) or stub
- [x] Add approved crates to Cargo.toml
- [ ] Quick license review for selected crates

---

## Core (Required)

### tauri

- Desktop app framework
- Frontend + backend bridge

### serde / serde_json

- Serialize event payloads
- Store structured JSON in SQLite

### uuid

- Generate IDs for sessions, events, checkpoints, artifacts

### chrono OR std::time

- Timestamp generation
- Prefer epoch milliseconds

---

## Database

### sqlx (Current Choice)

Why:

- Simple
- Already used in the codebase
- Supports SQLite well
- Widely used
- Good for MVP scale

Used for:

- Session storage
- Event logs
- Checkpoint summaries
- AI outputs

Alternative:

- rusqlite (if a lighter, sync-only client is needed later)

---

## File System & Paths

### tauri::api::path

- Resolve app data directory safely

### std::fs

- Create directories
- Write screenshot / report files

---

## Screenshot Capture (Choose Later)

Recommended options (platform dependent):

- scrap (cross-platform screen capture)
- xcap (modern, simpler API)

NOTE:
Screenshot capture can be stubbed initially.
Implement minimal working capture first.

---

## Input Recording (If Needed Later)

### rdev

- Global keyboard/mouse listener
- Use only if system-wide recording is required

For MVP:

- Prefer DOM-level recording instead

---

## LLM Integration

No specific Rust SDK required.

Use:

- HTTP client (reqwest or ureq)
- Local LLM endpoint (LM Studio)
- Or cloud API (Gemini / OpenAI-compatible)

Rules:

- LLM is stateless
- Keep prompts small
- Store outputs in SQLite

---

## Image / Artifact Handling

### image

- Encode/decode PNG/JPEG
- Resize if needed

Binary files are saved to filesystem only.

---

## Libraries Explicitly NOT Needed in MVP

- ORM frameworks
- Full text search engines
- Vector databases
- OCR / computer vision
- Streaming pipelines

---

## Acceptance Criteria

- All dependencies are stable and well-documented
- No unnecessary async complexity
- Easy for LLMs to reason about code structure
