# FEATURE 07 — AI Test Generation

## Objective

Generate negative and exploratory test cases using an LLM, expanding from existing positive/negative cases.


## Input

- Session goal
- Session type (browser | api)
- Checkpoint summary
- Existing positive + negative test cases
- Entities and risks
- API endpoints/params (for API sessions)


## Output

- negative_cases
- edge_cases
- exploratory_charters
- api_gap_checks (missing status codes, auth failures, validation gaps)

## Workflow Triggers

### Browser Sessions

- User clicks "AI Explore" after recording the positive flow with a post-submit result observed.
- Tauri runs suggested actions and records resulting events.
- Progress and results stream over WebSocket to React UI tabs.

### API Sessions

- User clicks "Generate Test Cases" after configuring the base request.
- Tauri executes generated edge cases and stores results.
- Progress and results stream over WebSocket to React UI tabs.


## Table

test_cases

## Rules

- Treat the LLM as stateless
- Persist all generated output to the database
- Expand negatives by referencing existing positive + negative cases
- Store derived cases with source_session_id or source_case_id metadata
- Require a post-submit result observed before generating cases


## Acceptance Criteria

- Test checklist generated successfully
- Negative cases reference existing coverage
- API gap checks generated for API sessions
- Can be executed per checkpoint

