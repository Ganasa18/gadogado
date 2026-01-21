# Model Distillation Feature

## Overview

The Model Distillation feature enables offline-first model training, evaluation, and export for fine-tuning and knowledge distillation workflows.

## Directory Structure

```
src/features/model-distillation/
├── pages/              # UI pages
│   ├── SetupTab.tsx    # Model configuration
│   ├── TrainTab.tsx    # Training monitoring
│   ├── EvaluateTab.tsx # Evaluation & metrics
│   └── ExportTab.tsx   # Export & version management
├── components/         # Shared UI components
│   ├── UI.tsx          # Reusable components (Card, Button, etc.)
│   └── index.ts
├── api/                # API clients
│   └── index.ts        # Tauri command wrappers
├── hooks/              # Custom React hooks
│   └── index.ts        # Data fetching hooks
├── types.ts            # TypeScript types
└── index.ts            # Feature exports

# Store
```
src/store/modelDistillation.ts
```
Zustand store for managing model-distillation state (training config, active session, etc.)
```

## Pages

### SetupTab (`/model-destilation/setup`)
Configure model training parameters:
- Select student/teacher models
- Choose training method (Fine-tune/KD/Hybrid)
- Set hyperparameters (epochs, batch size, learning rate, temperature, alpha)
- Configure dataset settings

### TrainTab (`/model-destilation/train`)
Monitor training in real-time:
- Start/pause/stop training
- View progress bar and epoch tracking
- Real-time metrics (loss, learning rate)
- Resource usage (CPU, RAM, ETA)
- Training logs with step-by-step progress

### EvaluateTab (`/model-destilation/evaluate`)
Compare model versions:
- Select candidate and baseline versions
- Choose evaluation dataset
- View metrics comparison (accuracy, F1, BLEU, etc.)
- See improvement/regression indicators
- Access evaluation history

### ExportTab (`/model-destilation/export`)
Export and manage versions:
- Select model version
- Choose export format (LoRA Adapter, Merged Model, GGUF)
- Configure export path
- Promote versions to active
- Rollback to previous versions
- View all versions with details

## API Usage

### Example: Create a training run

```typescript
import { ModelDistillationAPI } from "@/features/model-distillation/api";

const run = await ModelDistillationAPI.createTrainingRun({
  studentModelId: "phi3-mini-4k",
  method: "hybrid",
  hyperparams: {
    epochs: 5,
    batchSize: 4,
    learningRate: 0.0001,
    temperature: 3.0,
    alpha: 0.7,
  },
});
```

### Example: Use custom hook

```typescript
import { useTrainingRun, useTrainingLogs } from "@/features/model-distillation/hooks";

function TrainingMonitor({ runId }: { runId: string }) {
  const { run, loading, start, pause, cancel } = useTrainingRun(runId);
  const { logs } = useTrainingLogs(runId, true);

  if (loading) return <div>Loading...</div>;

  return (
    <div>
      <button onClick={start}>Start</button>
      <button onClick={pause}>Pause</button>
      <button onClick={cancel}>Cancel</button>
      <ul>
        {logs.map(log => (
          <li key={log.logId}>
            Epoch {log.epoch}, Loss: {log.loss}
          </li>
        ))}
      </ul>
    </div>
  );
}
```

## Components

### Card
Wrap content in a styled card with optional icon.

```typescript
import { Card } from "@/features/model-distillation/components";

<Card title="Training Control" icon={Activity}>
  <button onClick={start}>Start</button>
</Card>
```

### MetricCard
Display a metric with label and value.

```typescript
import { MetricCard } from "@/features/model-distillation/components";

<MetricCard
  label="Current Loss"
  value={loss}
  color="text-blue-500"
/>
```

### ProgressBar
Show progress with optional label.

```typescript
import { ProgressBar } from "@/features/model-distillation/components";

<ProgressBar value={3} max={5} label="Epoch" color="bg-blue-500" />
```

### Button
Styled button with variants.

```typescript
import { Button } from "@/features/model-distillation/components";

<Button variant="primary" onClick={start} icon={Play}>
  Start Training
</Button>
```

### StatusBadge
Display status with icon and color.

```typescript
import { StatusBadge } from "@/features/model-distillation/components";

<StatusBadge status="completed" text="Training Complete" />
```

## Backend Commands

The following Tauri commands need to be implemented in Rust:

### Corrections
- `md_create_correction`
- `md_get_corrections`
- `md_get_correction`
- `md_update_correction`
- `md_delete_correction`

### Training
- `md_create_training_run`
- `md_get_training_runs`
- `md_get_training_run`
- `md_start_training`
- `md_pause_training`
- `md_resume_training`
- `md_cancel_training`
- `md_get_training_logs`

### Model Versions
- `md_get_model_versions`
- `md_get_model_version`
- `md_get_active_version`
- `md_promote_version`
- `md_rollback_version`

### Export & Evaluation
- `md_export_model`
- `md_evaluate_version`
- `md_get_evaluation_metrics`

### Datasets
- `md_get_datasets`
- `md_get_dataset`
- `md_create_dataset`

### Artifacts
- `md_get_run_artifacts`
- `md_delete_run_artifact`
- `md_cleanup_old_runs`

## Theme Integration

All pages use the same theme colors as the rest of the app:

- `bg-app-bg` - Main background
- `bg-app-card` - Card background
- `border-app-border` - Border color
- `text-app-text` - Primary text
- `text-app-subtext` - Secondary text
- `text-blue-400` - Accent color for icons
- `text-app-success` - Success messages
- `text-app-success-dim` - Success background

## State Management

### Using the Zustand Store

```typescript
import { useModelDistillationStore } from "@/store/modelDistillation";

function MyComponent() {
  const {
    studentModel,
    setStudentModel,
    trainingMethod,
    setTrainingMethod,
    activeRunId,
  } = useModelDistillationStore();

  return (
    <div>
      <select value={studentModel} onChange={(e) => setStudentModel(e.target.value)}>
        <option value="phi3-mini-4k">Phi-3 Mini</option>
        <option value="llama-3-8b">Llama-3 8B</option>
      </select>
      <div>Active Run: {activeRunId || "None"}</div>
    </div>
  );
}
```

## Next Steps

1. Implement Rust backend commands in `src-tauri/`
2. Set up SQLite database with schema from `agent-md/model-destilation/schema_v2.sql`
3. Implement Python training core with KD/fine-tuning pipeline
4. Add real-time log streaming from Python to frontend
5. Connect UI to actual backend APIs
6. Add error handling and loading states
7. Implement export functionality (GGUF conversion, etc.)
