# QA Recording Modes & Configuration

## Overview

The QA automation system now supports two recording modes with configurable delay for more structured test case creation:

## Scope

- Recording modes apply to browser sessions only
- API sessions use request capture and replay without auto/manual toggles

1. **Auto Mode** - Continuous recording of all interactions
2. **Manual Mode** - Step-by-step recording with explicit control

## Features

### 1. Recording Modes

#### Auto Mode (Default)
- **Behavior**: Automatically records all user interactions in the iframe
- **Use Case**: Quick exploratory testing, recording full workflows
- **How it works**:
  1. Click "Start Record"
  2. All clicks, inputs, and form submissions are recorded automatically
  3. Click "Stop Record" to finish

#### Manual Mode
- **Behavior**: Record one event at a time with explicit confirmation
- **Use Case**: Structured test cases, precise step documentation, avoiding accidental events
- **How it works**:
  1. Toggle to "Manual" mode before starting
  2. Click "Start Record"
  3. Click "Record Next" to arm the recorder
  4. Perform ONE action (click, type, etc.)
  5. Event is recorded and recorder disarms
  6. Repeat steps 3-5 for each step
  7. Click "Stop Record" to finish

### 2. Recording Delay

- **Purpose**: Add delay before recording each event
- **Range**: 0ms - 5000ms (0-5 seconds)
- **Default**: 500ms
- **Use Case**:
  - Prevent accidental rapid clicks
  - Give time to verify before recording
  - Match real user interaction timing
  - Better for screenshot/video capture synchronization

## UI Controls

### Before Recording (Idle State)

```
┌─────────────────────────────────────────────────────┐
│ [Auto] [Manual]  Delay: [500] ms  [Start Record]   │
└─────────────────────────────────────────────────────┘
```

**Controls:**
- **Mode Toggle**: Switch between Auto/Manual mode
- **Delay Input**: Set delay in milliseconds (0-5000)
- **Start Record**: Begin recording session

### During Recording - Auto Mode

```
┌─────────────────────────────────────────────────────┐
│ [Recording...]  [Stop Record]  [End Session]        │
└─────────────────────────────────────────────────────┘
```

**Behavior:**
- All interactions are recorded automatically with configured delay
- Toast notification on first interaction: "Recording started (500ms delay)"

### During Recording - Manual Mode

```
┌─────────────────────────────────────────────────────┐
│ [Record Next]  [Recording...]  [Stop Record]        │
└─────────────────────────────────────────────────────┘
```

**Button States:**
- **[Record Next]** - Click to arm recorder (blue)
- **[Ready...]** - Armed, waiting for user action (blue, pulsing)
- After event: Automatically disarms, shows toast: "Event recorded. Click 'Record Next' to capture another."

## Implementation Details

### Store State (src/store/qaSession.ts)

```typescript
interface QaSessionState {
  recordingMode: "auto" | "manual";
  recordingDelay: number; // milliseconds
  isRecordingArmed: boolean; // for manual mode
  setRecordingMode: (mode: RecordingMode) => void;
  setRecordingDelay: (delay: number) => void;
  setIsRecordingArmed: (armed: boolean) => void;
}
```

### Recording Logic (src/hooks/useQaEventRecorder.ts)

#### Delay Implementation

```typescript
const recordEventWithDelay = (payload: QaEventPayload) => {
  const delay = recordingDelay || 0;
  recordingDelayTimeoutRef.current = window.setTimeout(() => {
    invoke("qa_record_event", { event: payload, sessionId });
  }, delay);
};
```

#### Manual Mode Logic

```typescript
const recordEvent = (payload: QaEventPayload) => {
  // In manual mode, only record if armed
  if (recordingMode === "manual" && !isRecordingArmed) {
    console.log("[QA Recorder] Manual mode: event ignored (not armed)");
    return;
  }

  recordEventWithDelay(payload);

  // In manual mode, disarm after recording one event
  if (recordingMode === "manual") {
    setIsRecordingArmed(false);
    addToast("Event recorded. Click 'Record Next' to capture another.", "success");
  }
};
```

## Usage Examples

### Example 1: Quick Exploratory Test (Auto Mode)

**Scenario**: Testing login flow quickly

1. Set delay to 500ms
2. Keep mode as "Auto"
3. Click "Start Record"
4. Perform actions naturally:
   - Type username
   - Type password
   - Click login button
5. Click "Stop Record"

**Result**: All actions recorded with 500ms delay, smooth test case

### Example 2: Structured Test Case (Manual Mode)

**Scenario**: Creating precise test steps for documentation

1. Set delay to 1000ms (1 second)
2. Switch to "Manual" mode
3. Click "Start Record"
4. **Step 1**: Click "Record Next" → Click username field → Event recorded
5. **Step 2**: Click "Record Next" → Type "student" → Event recorded
6. **Step 3**: Click "Record Next" → Click password field → Event recorded
7. **Step 4**: Click "Record Next" → Type password → Event recorded
8. **Step 5**: Click "Record Next" → Click submit button → Event recorded
9. Click "Stop Record"

**Result**: Precise, step-by-step test case with clear separation between actions

### Example 3: Testing Complex Form (Manual Mode)

**Scenario**: Form with dynamic fields that need careful documentation

1. Set delay to 800ms
2. Switch to "Manual" mode
3. Click "Start Record"
4. Record each field interaction separately:
   - Select country dropdown
   - Wait for state field to appear
   - Select state
   - Fill address
   - Check terms checkbox
   - Submit

**Result**: Clear documentation of form behavior and field dependencies

## Best Practices

### When to Use Auto Mode
- ✅ Quick exploratory testing
- ✅ Recording long workflows
- ✅ Testing navigation flows
- ✅ When you know the exact steps

### When to Use Manual Mode
- ✅ Creating structured test documentation
- ✅ Testing complex interactions with timing considerations
- ✅ Recording demos or tutorials
- ✅ When you need to verify each step before recording
- ✅ Avoiding accidental event recording

### Recommended Delays

| Scenario | Recommended Delay | Reason |
|----------|------------------|---------|
| Fast typing/clicking | 300-500ms | Prevents double-clicks |
| Normal interaction | 500-800ms | Natural user timing |
| Complex forms | 800-1200ms | Time to see changes |
| Demo recording | 1000-1500ms | Easier to follow |
| Slow/thoughtful testing | 1500-3000ms | Time to verify each step |

## Troubleshooting

### Issue: Events recorded too fast in Auto mode
**Solution**: Increase delay to 800ms or higher

### Issue: Need to skip some actions
**Solution**: Use Manual mode and only arm recorder for desired actions

### Issue: Accidentally recorded wrong event
**Solution**:
1. Stop recording
2. Delete unwanted event from timeline
3. Resume recording

### Issue: Manual mode button not responding
**Solution**:
1. Check if recording is active (should see "Recording..." status)
2. Ensure you clicked "Record Next" before performing action
3. Check browser console for errors

## Technical Notes

### Cross-Origin Support
Both modes work with:
- ✅ Same-origin iframes (direct DOM access)
- ✅ Cross-origin iframes via proxy (postMessage)
- ✅ External websites (proxied through localhost:3001)

### Event Timeout Cleanup
All pending delay timeouts are properly cleaned up when:
- Recording stops
- Session ends
- User navigates away
- Component unmounts

### Performance
- Delay timers are efficiently managed per event
- Manual mode reduces event volume significantly
- No performance impact when not recording

## Future Enhancements

Potential improvements:
- [ ] Keyboard shortcuts for manual mode (e.g., Ctrl+Space to arm)
- [ ] Batch recording mode (record N events then stop)
- [ ] Variable delay per event type
- [ ] Pause/Resume functionality
- [ ] Undo last recorded event
- [ ] Custom event annotations during manual recording

## Related Documentation

- [01_OVERVIEW.md](./01_OVERVIEW.md) - QA automation overview
- [02_SESSION_MANAGEMENT.md](./02_SESSION_MANAGEMENT.md) - Session creation
- [03_EVENT_RECORDER.md](./03_EVENT_RECORDER.md) - Event recording details
- [CHECKPOINT_PROGRESS.md](./CHECKPOINT_PROGRESS.md) - Implementation progress
