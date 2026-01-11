# QA Automation Troubleshooting Guide

## Common Issues & Solutions

### 1. CORS Errors with Localhost Applications

#### Symptom
```
Access to fetch at 'http://localhost:3000/...' from origin 'http://localhost:3001'
has been blocked by CORS policy: No 'Access-Control-Allow-Origin' header is present
```

#### Cause
When proxying localhost applications (e.g., React dev server on port 3000), CORS policies may block resource loading.

#### Solution A: Use Direct URL (No Proxy)
If the target application is on `http://localhost:XXXX`, you can access it directly without proxy:
1. Use the direct URL in session notes: `{"preview_url": "http://localhost:3000"}`
2. Our system will automatically use proxy for cross-origin
3. But for better compatibility with dev servers, consider Solution B

#### Solution B: Configure Dev Server CORS
For React/Next.js apps, add CORS headers to your dev server:

**Next.js** (`next.config.js`):
```javascript
module.exports = {
  async headers() {
    return [
      {
        source: '/:path*',
        headers: [
          { key: 'Access-Control-Allow-Origin', value: '*' },
          { key: 'Access-Control-Allow-Methods', value: 'GET,POST,OPTIONS' },
          { key: 'Access-Control-Allow-Headers', value: '*' },
        ],
      },
    ];
  },
};
```

**Vite** (`vite.config.js`):
```javascript
export default {
  server: {
    cors: true,
  },
};
```

**Create React App**: Use [CORS proxy](https://www.npmjs.com/package/cors-anywhere) or modify package.json

#### Solution C: Disable Browser Security (Development Only)
⚠️ **WARNING: Only for development/testing**

Launch Chrome with disabled web security:
```bash
chrome.exe --disable-web-security --user-data-dir="C:/chrome-dev-session"
```

### 2. React Hydration Errors

#### Symptom
```
Warning: Prop `%s` did not match. Server: %s Client: %s
Uncaught Error: Text content does not match server-rendered HTML
```

#### Cause
These are React/Next.js application errors, not QA recorder errors. They occur when:
- Server-rendered HTML doesn't match client-side rendering
- Dynamic content changes between server and client
- Time/date sensitive content

#### Solution
**These errors are in the TARGET application, not our QA system.**

To fix in target app:
1. Use `useEffect` for client-only rendering
2. Ensure server and client render same HTML
3. Use `suppressHydrationWarning` prop for intentional mismatches
4. Check for browser-specific APIs used during SSR

**For QA Recording:** These errors don't affect event recording. You can safely ignore them and continue recording.

### 3. Events Not Being Recorded

#### Symptom
Recording status shows "Recording..." but no events appear in timeline.

#### Diagnosis Steps

1. **Check Browser Console**
```javascript
// Should see these logs:
[QA Recorder] Recording session started: <id>
[QA Recorder] Frame found, src: <url>
[QA Recorder] Iframe listeners attached successfully
// or
[QA Recorder] Iframe recorder script ready (for cross-origin)
```

2. **Check Manual Mode State**
```javascript
// If in Manual mode, check:
- Is "Record Next" button clicked?
- Does button show "Ready..."?
// If not, click "Record Next" before performing action
```

3. **Check Recording Is Armed**
```javascript
// Console should show after first click:
[QA Recorder] Recording event: click {...}
```

#### Solutions

**A. Iframe Not Found**
```
[QA Recorder] Frame element not found in DOM
```
- Ensure preview URL is valid
- Check session has `preview_url` in notes
- Reload the session page

**B. Cross-Origin Access Denied**
```
[QA Recorder] Error: contentDocument is null
[QA Recorder] Cross-origin iframe detected
```
- This is expected for external sites
- Check if proxy is working: `http://localhost:3001/api/qa/proxy?url=...`
- Look for: `[QA Recorder Inject] Script loaded via proxy`

**C. Manual Mode Not Armed**
```
[QA Recorder] Manual mode: event ignored (not armed)
```
- Click "Record Next" button before performing action
- Wait for button to show "Ready..." state
- Then perform ONE action

**D. Recording Delay Too High**
- If delay is >2000ms, events may seem slow to appear
- Reduce delay in UI controls
- Default 500ms is recommended

### 4. Proxy Server Not Responding

#### Symptom
```
Failed to fetch URL: Connection refused
```

#### Check Actix Server Status
1. Look for backend logs:
```
[INFO] actix_server::server: starting service: "actix-web-service-127.0.0.1:3001"
```

2. Test proxy endpoint:
```bash
curl "http://localhost:3001/api/qa/proxy?url=https://example.com"
```

#### Solutions
- Restart Tauri app
- Check port 3001 is not in use: `netstat -ano | findstr :3001`
- Check firewall not blocking localhost:3001

### 5. Script Injection Not Working

#### Symptom (Cross-Origin)
```
[QA Recorder Inject] Script loaded via proxy  // ✗ Missing
```

#### Diagnosis
1. View proxied HTML source (browser DevTools > Sources)
2. Check for injected script in `<head>`
3. Look for: `// QA Recorder Injectable Script - Injected by Proxy`

#### Solutions
- Check backend logs for "Successfully proxied and injected recorder script"
- Verify HTML has `<head>` or `<body>` tag
- Try different URL (some sites may block injection)

### 6. Performance Issues

#### Symptom
- Browser becomes slow during recording
- UI freezes
- Events take long to process

#### Solutions

**A. Reduce Event Volume**
- Use Manual mode instead of Auto mode
- Increase recording delay to 1000-1500ms
- Avoid rapid clicking/typing

**B. Limit Session Length**
- Stop recording after 50-100 events
- Create multiple smaller sessions instead of one large session
- End and start new session periodically

**C. Close Unused Sessions**
- Only keep one recording session active
- End completed sessions
- Don't leave multiple sessions open

### 7. Selector Not Capturing Correctly

#### Symptom
Event recorded but selector seems wrong or too generic.

#### Current Selector Priority
```
1. data-testid
2. data-purpose
3. id
4. name
5. aria-label
6. role
7. :nth-of-type() path (fallback)
```

#### Solutions

**A. Add Test IDs to Your App**
```html
<!-- Recommended -->
<button data-testid="login-button">Login</button>

<!-- Also good -->
<input data-purpose="username-input" />
<button id="submit-btn">Submit</button>
```

**B. Use Unique IDs**
```html
<!-- Good -->
<div id="user-profile-header">

<!-- Bad -->
<div id="header"> <!-- Too generic -->
```

**C. Add Semantic Attributes**
```html
<button aria-label="Close dialog">×</button>
<input name="email" aria-label="Email address">
```

### 8. Password Fields Not Masked

#### Symptom
Password values visible in event data.

#### Check
1. View event in timeline
2. Check "value" field
3. Should show `"[masked]"` for passwords

#### Masking Rules
Passwords are auto-masked if:
- Input type is `password`
- Name/id/aria-label contains "password" (case-insensitive)

#### Solutions
- Ensure password fields have `type="password"`
- Or include "password" in name/id/label
- Example: `<input name="user_password" />`

## Browser Compatibility

### Supported Browsers
- ✅ Chrome/Chromium (Recommended)
- ✅ Edge (Chromium-based)
- ⚠️ Firefox (Limited - Some features may not work)
- ❌ Safari (Not supported in Tauri)

### Known Limitations

**Firefox:**
- `CSS.escape()` may not be available (fallback implemented)
- Some event timing differences
- Cross-origin messaging may be slower

**Tauri WebView:**
- Uses system WebView (Edge on Windows, WebKit on macOS)
- Some modern JS features may need polyfills
- File upload requires special handling

## Debug Mode

### Enable Verbose Logging

Add to browser console:
```javascript
localStorage.setItem('qa-recorder-debug', 'true');
```

Reload page, then check for detailed logs:
```
[QA Recorder] [DEBUG] Event captured: {...}
[QA Recorder] [DEBUG] Selector built: button[data-testid="login"]
[QA Recorder] [DEBUG] Delay applied: 500ms
```

### Disable Debug Mode
```javascript
localStorage.removeItem('qa-recorder-debug');
```

## Getting Help

### Information to Provide

When reporting issues, include:

1. **Browser Console Output**
   - All `[QA Recorder]` logs
   - Any error messages
   - Full stack traces

2. **Backend Logs**
   - Terminal output
   - Look for `[QA]` and `[QA Proxy]` entries

3. **Environment**
   - OS version
   - Tauri app version
   - Target URL (if public)

4. **Steps to Reproduce**
   ```
   1. Create QA session with URL: ...
   2. Set mode to: Auto/Manual
   3. Set delay to: 500ms
   4. Click "Start Record"
   5. Perform action: ...
   6. Observe: ...
   ```

5. **Screenshots**
   - QA session page
   - Browser DevTools console
   - Event timeline (if events recorded)

### Useful Debug Commands

**Check if recording is active:**
```javascript
// Browser console
window.__QA_RECORDER_INJECTED__ // should be true if injected
```

**Manually trigger event:**
```javascript
// Browser console (in iframe if cross-origin)
window.parent.postMessage({
  type: 'qa-recorder-event',
  payload: {
    eventType: 'click',
    selector: 'button',
    elementText: 'Test',
    url: window.location.href,
  }
}, '*');
```

**Check frame access:**
```javascript
// Parent window console
const frame = document.querySelector('[data-qa-preview-frame]');
console.log(frame.contentDocument); // null = cross-origin
```

## Related Documentation

- [01_OVERVIEW.md](./01_OVERVIEW.md) - System overview
- [03_EVENT_RECORDER.md](./03_EVENT_RECORDER.md) - Recording details
- [04_RECORDING_MODES.md](./04_RECORDING_MODES.md) - Recording modes
- [CHECKPOINT_PROGRESS.md](./CHECKPOINT_PROGRESS.md) - Current status
