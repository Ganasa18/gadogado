# Backend PRD - gadogado [READONLY]

## Purpose

Provide a secure, native backend service for LLM integration, credential management, logging, and frontend communication via Tauri.

## Responsibilities

- LLM gateway (local and cloud providers)
- Configuration loading and validation
- Secrets handling via OS keychain
- Optional local history storage
- Local-only logging and diagnostics

## Core Features

- LLM routing
  - Local: `http://localhost:1234/v1` (OpenAI-compatible)
  - Cloud: OpenAI, Google, DLL endpoints
- Secure credential storage
  - Prefer OS keychain via `keyring` crate
  - Avoid custom crypto in application code
- Input validation
  - Strongly typed structs + validation in Rust
- Rate limiting
  - Example: max 3 LLM requests per second per endpoint
- Optional history
  - Store translations/enhancements in SQLite via `sqlx`
- Configuration
  - Load settings with `figment` from local config
- Logging
  - Structured logs with `tracing` and `tracing-subscriber`

## Interfaces

### Tauri Commands (primary)

- `translate_text`
- `enhance_prompt`
- `sync_config`
- `sync_languages`
- `sync_shortcuts`

### Local HTTP (debug only)

- `POST /api/translate`
- `POST /api/enhance`
- `POST /api/models`
- `GET /api/logs`

## Error Handling

- Use RFC 7807 (Problem Details for HTTP APIs) for local HTTP errors.
- Normalize error responses for Tauri commands.

## Non-Functional Requirements

- Offline support when local LLM is available
- No hardcoded secrets
- Lightweight (< 200 MB RAM at idle)
- Predictable latency with timeouts and retries

## Recommended Backend Structure

See `agent-md/Folder-Structure.md` for the detailed folder layout.

## Testing Strategy

- Unit tests for domain logic and validation
- Integration tests for LLM gateway and database operations
- Mock providers for deterministic test runs
