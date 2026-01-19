# Backend Requirements for Model Distillation

## Training Data Generation
The new "Training Data Generator" in `SetupTab` requires a way to programmatically invoke LLM models (both the "Teacher" model from the configured provider and the "Student" model being trained/local).

### Required Tauri Command: `llm_chat`
A new Tauri command `llm_chat` is needed to handle chat completion requests.

**Signature:**
```rust
#[tauri::command]
async fn llm_chat(config: LlmConfig, messages: Vec<Message>) -> Result<String, String>
```

**Types:**
```typescript
interface LlmConfig {
  provider: "local" | "ollama" | "llama_cpp" | "openai" | "gemini" | "dll";
  model: string;
  apiKey?: string; // Optional (for cloud providers)
  baseUrl?: string; // Optional (for openai compatible endpoints)
  maxTokens?: number;
  temperature?: number;
}

interface Message {
  role: "user" | "assistant" | "system";
  content: string;
}
```

**Functionality:**
1.  **Teacher Model:** When `provider` is `openai` or `gemini`, it should make an HTTP request to the respective API.
2.  **Student Model:** When `provider` is `local` (or whatever the student model format uses, e.g., `llama_cpp`), it should invoke the local model inference engine (e.g., loading the GGUF model specified by `config.model` path).
3.  **Output:** Return the generated text string directly.

### Alternative (HTTP Endpoint)
If a Tauri command is difficult, an HTTP endpoint `POST /api/chat` accepting the same JSON body could work, provided the frontend `apiClient` is updated to call it.

## Current status in Frontend
- `SetupTab.tsx` currently attempts to import `invoke` from `@tauri-apps/api/core` and call `invoke("llm_chat", ...)` inside `handleGenerateTrainingData`.
- This will fail until the backend command is registered.
