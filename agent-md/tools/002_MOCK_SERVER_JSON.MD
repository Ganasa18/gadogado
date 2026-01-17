# MOCK SERVER JSON

## Goal
Provide a lightweight local mock server that serves JSON responses with flexible request matching for rapid API prototyping and testing.

## Core Features
- Support HTTP methods: GET, POST, PATCH, DELETE, OPTIONS.
- Match by path, query params, headers, and body.
- Support JSON, x-www-form-urlencoded, and multipart form data.
- Custom response: status code, headers, body, delay.
- Import cURL commands to create mock routes quickly.
- Live request log with replay and diff tools.

## Request Matching Rules
- Match priority: method + path, then query, then headers, then body.
- Match modes: exact, contains, regex (optional).
- Body match: raw JSON compare, subset match, or JSONPath rules.

## Response Templates
- Static JSON response.
- Dynamic placeholders (e.g., request headers, query values, timestamps).
- Optional response delay (ms) for latency simulation.

## Admin API (Local Only)
- Create, update, delete, and list mock routes.
- Enable/disable a mock without deleting it.
- Export and import all mocks as JSON.

## UI (If applicable)
- Route list with status toggles.
- Request log with filters.
- Quick cURL import input.

## Checkpoints

### Checkpoint 1: Core Server
- [ ] Start server and listen on configurable port
- [ ] Register mock routes from JSON config
- [ ] Match method + path
- [ ] Return custom status and JSON body

### Checkpoint 2: Matching Depth
- [ ] Query param matching
- [ ] Header matching
- [ ] Body matching for JSON
- [ ] Match modes (exact/contains)

### Checkpoint 3: Form Data + cURL
- [ ] x-www-form-urlencoded parsing
- [ ] Multipart form data parsing
- [ ] cURL import to generate a mock route

### Checkpoint 4: Tooling
- [ ] Request log viewer
- [ ] Export/import mocks
- [ ] Response delay simulation
- [ ] Admin API endpoints

## Acceptance Criteria
- Mocks return deterministic responses for matching requests.
- Non-matching requests return clear errors (404 or 501).
- cURL import produces equivalent mock configuration.
- Logs capture method, path, status, latency, and timestamp.
