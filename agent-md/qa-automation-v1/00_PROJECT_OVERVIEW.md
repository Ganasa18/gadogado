# MVP AI-assisted QA Recorder — Project Overview [READONLY]

## Goal

Build a minimal QA tool using Tauri that:

- records happy-path (positive case) web flows
- supports API sessions for request/response coverage
- splits long sessions to stay within LLM context limits
- uses SQLite as persistent memory
- generates negative cases and exploratory checklists via LLMs
- replays recorded flows to validate correctness


## Constraints

- Free/local LLMs or hosted LLM APIs (LM Studio, OpenAI, etc.)

- Limited context window (4k–16k tokens)
- LLM is stateless
- No vision-based analysis in MVP

## Core Principles

- Database is the source of truth
- LLM is a generator, not a memory store
- Features are built incrementally and independently

## Main Workflow

### Browser Session (Positive Flow)

1. User starts a browser session in Tauri.
2. Tauri opens a visible Playwright instance for the target URL.
3. User records the happy-path flow in the embedded browser.
4. User clicks "AI Explore" to request AI-suggested actions.
5. Tauri executes suggested actions and records resulting events.
6. WebSocket streams progress + results to React UI tabs in real time.

### API Session (Edge Coverage)

1. User starts an API session and configures the base request.
2. User clicks "Generate Test Cases" to produce AI edge cases.
3. Tauri executes generated cases and saves results to storage.
4. WebSocket streams progress + results to React UI tabs in real time.

## Build Order


1. Bootstrap & Storage
2. Session Manager
3. Event Recorder (Web + API)
4. Replay System
5. Screenshot Capture
6. Checkpoint System
7. Event Chunking & Summary
8. AI Test Generation
9. Deduplication & Priority
10. Report Export

