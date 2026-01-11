# FEATURE 05 — Checkpoint System

## Objective

Prevent LLM context overflow by splitting sessions.

## Checkpoint Triggers

- Manual (QA action)
- Form submit
- Navigation change
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

- Checkpoints are created correctly
- Events are associated with checkpoints
