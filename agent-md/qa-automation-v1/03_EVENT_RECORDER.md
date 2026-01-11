# FEATURE 03 — DOM Event Recorder

## Objective

Record user actions inside the Tauri webview.

## Event Types

- click
- input (debounced)
- submit
- navigation / route change

## Event Payload

- event_type
- selector (prefer data-testid)
- element_text
- value (mask passwords)
- url
- meta_json

## Rules

- Do not log every keystroke
- Do not log sensitive data

## Frontend

- Recording panel lives on `/qa/session/:id`
- Start Record button enabled only when the session is not ended
- Event list fetches by `session_id` and live-updates in chronological order

## Acceptance Criteria

- Events stored sequentially in the database
- Selectors are consistent and reusable

## Checklist

- [x] Integrate session lifecycle data created success with event
- [~] record event on progress
