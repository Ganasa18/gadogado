# FEATURE 06 — Event Chunking & Summary

## Objective

Compress event data to stay within token limits.

## Rules

- Maximum 20–50 events per chunk
- Split on submit or navigation only after post-submit result observed
- Use compact text format, not raw JSON


## Summary Output

- Bullet-point summary
- Entities touched (fields, buttons, routes)
- Anomalies (errors, validation issues)

## Acceptance Criteria

- Summaries stored in the database
- Raw long event lists are never sent to LLM
- Summaries include post-submit results when applicable

