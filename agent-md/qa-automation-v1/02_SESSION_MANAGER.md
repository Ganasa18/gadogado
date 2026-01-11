# FEATURE 02 — Session Manager

## Objective

Manage the QA session lifecycle.

## Table

sessions:

- id
- title
- goal
- is_positive_case
- started_at
- ended_at

## Backend Commands

- qa_start_session
- qa_end_session
  - Use `add_log` for start/stop success and error paths so the UI log panel reflects QA actions.

## Frontend

- Title input
- Goal input (required)
- Positive case toggle
- Start / Stop button
- Session creation defaults `is_active = false`; recording does not auto-start
- History list rows navigate to `/qa/session/:id`
- Session detail uses `preview_url` from session metadata (goal-derived or `target_url` saved on creation)

## Acceptance Criteria

- Session stored in the database
- ended_at populated on stop
- session_id available globally

## Checklist

- [~] Integrate session lifecycle with Event Recorder (Feature 03)
- [x] Manual start/stop verified in app
- [x] List session

## Progress Notes

- Manual check: qa_start_session succeeds and qa_end_session succeeds.
- Event Recorder (Feature 03 & Feature 04) not started.
  Status: In progress (awaiting Event Recorder integration).
