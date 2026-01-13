# FEATURE 02 — Session Manager

## Objective

Manage the QA session lifecycle for browser and API runs.


## Table

sessions:

- id
- title
- goal
- session_type (browser | api)
- is_positive_case
- target_url (browser)
- api_base_url (api)
- auth_profile_json
- source_session_id (nullable, derived negatives)
- started_at
- ended_at


## Backend Commands

- qa_start_session
  - Inputs: session_type, is_positive_case, target_url or api_base_url, auth_profile_json, source_session_id
- qa_end_session
  - Use `add_log` for start/stop success and error paths so the UI log panel reflects QA actions.


## Frontend

- Session type selector (Browser | API)
- Title input
- Goal input (required)
- Positive case toggle
- Browser: target URL input for `preview_url`
- API: base URL + auth profile (headers/token) inputs
- Start / Stop button
- Session creation defaults `is_active = false`; recording does not auto-start
- History list rows navigate to `/qa/session/:id` and show session type badge
- Session detail uses `preview_url` for browser or an API session overview panel

## Session Flows

### Browser Session (Positive Flow)

- User starts a browser session and records the happy-path flow.
- Tauri opens a visible Playwright browser for recording.
- User clicks "AI Explore" to generate AI-suggested actions.
- Suggested actions execute and are recorded as events.

### API Session (Edge Coverage)

- User configures base request details and starts the API session.
- User clicks "Generate Test Cases" to produce AI edge cases.
- Tauri executes generated cases and stores results.
- Derived negative sessions track `source_session_id` metadata.

## Acceptance Criteria

- Session stored with session_type metadata
- ended_at populated on stop
- session_id available globally
- source_session_id stored for AI-derived negative sessions


## Checklist

- [~] Integrate session lifecycle with Event Recorder (Feature 03)
- [x] Manual start/stop verified in app
- [x] List session

## Progress Notes

- Manual check: qa_start_session succeeds and qa_end_session succeeds.
- Event Recorder (Feature 03 & Feature 04) not started.
  Status: In progress (awaiting Event Recorder integration).
