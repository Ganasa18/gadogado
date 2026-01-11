# FEATURE 08 — Deduplication & Priority

## Objective

Avoid duplicate test cases and assign priorities.

## Deduplication Rule

Hash based on:

- type
- normalized title
- entities involved

## Priority Levels

- P0: crash, auth failure, data loss
- P1: validation, boundary cases
- P2: UX or minor issues

## Acceptance Criteria

- No excessive duplicates
- Priority levels are consistent
