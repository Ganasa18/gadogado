# MVP AI-assisted QA Recorder — Project Overview [READONLY]

## Goal

Build a minimal QA tool using Tauri that:

- records happy-path (positive case) user flows
- splits long sessions to stay within LLM context limits
- uses SQLite as persistent memory
- generates negative cases and exploratory checklists via LLM

## Constraints

- Free or local LLMs (LM Studio)
- Limited context window (4k–16k tokens)
- LLM is stateless
- No vision-based analysis in MVP

## Core Principles

- Database is the source of truth
- LLM is a generator, not a memory store
- Features are built incrementally and independently

## Build Order

1. Bootstrap & Storage
2. Session Manager
3. Event Recorder
4. Screenshot Capture
5. Checkpoint System
6. Event Chunking & Summary
7. AI Test Generation
8. Deduplication & Priority
9. Report Export
