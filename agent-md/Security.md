# Security Guidelines - gadogado

## Core Principles

- Zero trust: treat all input as potentially malicious
- Least privilege: access only what is necessary
- Defense in depth: multiple layers of protection
- No secrets in code: secrets are stored at runtime via OS keychain

## OWASP Top 10 Mitigations (2021)

| Risk                           | Mitigation                                                              |
| ------------------------------ | ----------------------------------------------------------------------- |
| A01: Broken Access Control     | No external authentication; all access is local. No public endpoints.   |
| A02: Cryptographic Failures    | Use OS keychain (`keyring`). Use SQLCipher for DB encryption if needed. |
| A03: Injection                 | Use `sqlx` with parameterized queries only.                             |
| A04: Insecure Design           | DDD separation reduces side effects and enforces boundaries.            |
| A05: Security Misconfiguration | Secure defaults; no ports exposed beyond localhost.                     |
| A06: Vulnerable Components     | Regular scans with Dependabot and `cargo audit`.                        |
| A07: Identification Failures   | Not applicable (no user authentication).                                |
| A08: Software/Data Integrity   | Tauri validates bundle integrity; signed updates if enabled.            |
| A09: Logging and Monitoring    | Local-only logging; no external telemetry.                              |
| A10: SSRF                      | Allow outbound requests only to configured LLM endpoints.               |

## Input Validation and Sanitization

- Frontend: Zod schemas for all forms and inputs
- Backend: `validator` crate + custom sanitization
  - Limit prompt length (e.g., 4096 characters)
  - Strip control characters (U+0000 to U+001F)
  - HTML-escape content if needed

## Rate Limiting

- Example: max 5 requests per minute per LLM endpoint
- Return HTTP 429 on violation

## Error Handling

- Follow RFC 7807 for local HTTP errors
- Normalize Tauri error responses

## Data Handling

- Prompts remain local except for calls to user-configured LLM endpoints
- History storage is optional and user-cleared

## Logging

- Logs are local-only and visible in the app terminal
- Avoid logging API keys or full prompt content

## Secure Defaults Checklist

- Disable any public HTTP exposure
- Restrict CORS to localhost (debug only)
- Validate all input, even from the UI
- Enforce reasonable timeouts and retries
