# FEATURE 05 — Checkpoint System

## Objective

Prevent LLM context overflow by splitting sessions.

## Checkpoint Triggers

- Manual (QA action)
- Form submit (only after post-submit result observed)
- Navigation change (only after post-submit result observed, if applicable)
- Post-submit result observed (DOM change, success message, or API response)
- Idle > 10–15 seconds


## Table

checkpoints:

- id
- session_id
- seq
- title
- start_event_seq
- end_event_seq

## Acceptance Criteria

- Checkpoints are created only after post-submit results are observed
- Events are associated with checkpoints

