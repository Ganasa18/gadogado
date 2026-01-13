# FEATURE 03 — DOM Event Recorder

## Objective

Record user actions inside the Tauri webview and API sessions.


## Event Types

- click
- input (debounced)
- submit
- navigation / route change
- api_request
- api_response


## Event Payload

### Browser Payload

- event_type
- selector (prefer data-testid)
- element_text
- value (mask passwords)
- url
- meta_json

### API Payload

- event_type (api_request | api_response)
- url
- meta_json (method, headers, request_body, status_code, response_body_hash, timing_ms)


## Rules

- Do not log every keystroke
- Do not log sensitive data (passwords, tokens, PII)
- Truncate large request/response bodies in meta_json

## Frontend

- Recording panel lives on `/qa/session/:id`
- Start Record button enabled only when the session is not ended
- Event list fetches by `session_id` and live-updates in chronological order
- API sessions show method, endpoint, and response status in the list

## Replay System

### Commands

- qa_replay_session (session_id, mode, from_checkpoint_id?)
- qa_replay_checkpoint (checkpoint_id)

### Rules

- Browser replay uses selectors + values to re-run UI steps
- API replay re-issues HTTP requests using stored method/url/body
- Replay results are stored as replay_runs with status + error output

## Acceptance Criteria

- Events stored sequentially in the database
- Selectors are consistent and reusable
- Replay runs can report pass/fail

## Checklist

- [x] Integrate session lifecycle data created success with event
- [x] record event on progress
