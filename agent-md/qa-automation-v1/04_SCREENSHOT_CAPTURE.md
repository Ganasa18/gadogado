# FEATURE 04 — Screenshot Capture

## Objective

Store visual evidence for each recorded event.

## Flow

1. Event is logged
2. Screenshot is captured
3. Screenshot is linked to the event

## Backend

- Command: qa_capture_screenshot
- Save image to filesystem
- Insert artifact record into database

## Frontend

- Preview column loads `preview_url` in an embedded webview/iframe
- Full-screen toggle available for the preview
- Preview screenshot refreshes on new events and via manual capture

## Acceptance Criteria

- Screenshot file is valid
- Event has a screenshot_id reference
