# Frontend PRD - gadogado [READONLY]

## Purpose

Provide a responsive, minimal, and fast user interface for prompt translation and enhancement workflows.

## Core Workflows

- Translation: input text -> choose languages -> get translated output
- Enhancement: input prompt -> get improved version with better structure
- Shortcuts: fast actions for translate, popup, and enhance

## UI Features

- LLM provider and model selector
- API key management (never stored in localStorage)
- In-app terminal for local logs
- Tabs for General, Shortcuts, Tutorial, Feedback, History

## State and Data

- Global state: Zustand
- Forms and validation: React Hook Form + Zod
- HTTP: Axios for local backend API

## Non-Functional Requirements

- Responsive UI with async LLM calls
- Lightweight (< 100 MB RAM at idle)
- Offline-first when local LLM is available
- Private by default (no analytics)

## Recommended Frontend Structure

See `agent-md/Folder-Structure.md` for the detailed folder layout.

## Conventions

- Prefer feature-based grouping for complex flows
- Avoid `any` where possible; type UI events and API payloads
- Reuse shared UI components instead of duplicating patterns
