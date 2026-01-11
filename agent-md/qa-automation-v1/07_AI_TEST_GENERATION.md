# FEATURE 07 — AI Test Generation

## Objective

Generate negative and exploratory test cases using an LLM.

## Input

- Session goal
- Checkpoint summary
- Entities and risks

## Output

- negative_cases
- edge_cases
- exploratory_charters

## Table

test_cases

## Rules

- Treat the LLM as stateless
- Persist all generated output to the database

## Acceptance Criteria

- Test checklist generated successfully
- Can be executed per checkpoint
