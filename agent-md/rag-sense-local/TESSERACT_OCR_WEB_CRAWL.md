# Tesseract OCR Web Crawl

Build a desktop crawler that converts live web pages into clean Markdown using a fully local stack:
Playwright + Screenshot + Grayscale + Tesseract OCR + Stitching + Fassembed Embedding.

## Core Principles

- Screenshot-first, PDF-never: capture exactly what the user sees (no print CSS or hidden elements). PDF export is opt-in only.
- Grayscale preprocessing: convert every screenshot to grayscale before OCR to boost contrast and reduce noise.
- Local & offline by default: zero internet dependency, ideal for privacy-sensitive apps like gadogado or PromptBridge.
- Reproducible artifacts: each session outputs `out.md`, `links.json`, `manifest.json`, `tiles/`, and `embedding.bin`.

## Processing Pipeline

### 1 Capture

- Playwright (headless Chromium)
- Viewport: 1280 x 2000, `deviceScaleFactor = 2`, animations disabled
- Deterministic scroll with ~120px vertical overlap

### 2 Preprocessing

Convert each tile to grayscale in Rust using the `image` crate.

```rust
let gray = DynamicImage::ImageRgba8(img).grayscale();
```

### 3 OCR

Run Tesseract per tile:

```bash
tesseract tile.png stdout -l eng+ind --oem 1 --psm 6
```

### 4 Stitching

- Merge outputs with markers: `<!-- tile:i offset:y -->`
- Deduplicate overlapping lines via fuzzy matching

### 5 Embedding

Save `out.md`, then run:

```bash
fassembed embed --input out.md --output embedding.bin
```

### 6 Metadata

- `manifest.json`: engine, version, languages, viewport, timestamp
- `links.json`: extracted anchors, forms, meta (if DOM available)

## Tech Stack

- Backend: Rust + Playwright (subprocess), Tesseract CLI, Fassembed CLI
- OCR: Tesseract, grayscale input, multilingual (eng+ind)
- Embedding: Fassembed (local CLI)
- Storage: filesystem artifacts + AES-256 encrypted SQLite (metadata)
- Frontend: React (TypeScript), Tauri, Tailwind CSS, Lucide Icons
- Logging: real-time `add_log()` to frontend (no sensitive data)

## Tauri Integration

```rust
#[tauri::command]
async fn capture_url(url: String) -> Result<String, String> {
    // 1. Playwright: capture tiles
    // 2. Preprocess: grayscale
    // 3. Tesseract OCR per tile
    // 4. Stitch and write out.md
    // 5. Fassembed: embedding.bin
    // 6. Persist to encrypted SQLite + filesystem
    // 7. Return job_id or output path
}
```

Outputs are saved in the app data directory (e.g., `appDataDir`) and returned to the React frontend as file paths.

## QA & Development

- Run `cargo check` after every feature
- Test edge cases: cookie banners, lazy-loaded content, canvas-heavy sites
- Validate OCR accuracy and embedding determinism

## Advantages

- Lightweight, fast, and fully offline
- `.txt`/`.md` output works with Ollama, Llama.cpp, and local RAG
- Strong context isolation, auditability, and traceability
- Ready for AI-assisted QA automation, bug hunting, or coding assistants
