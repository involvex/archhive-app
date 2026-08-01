# Enhanced Streaming & Browse Experience Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enhance the streaming experience with keyboard navigation (← →) in the library player, add a Watch button to browse SceneCards for direct playback, add a Watch Next button during playback, and add an option to disable LAN bearer token authentication.

**Architecture:** Extend the existing `ScenePlayerDialog` to accept a scene list and index for sequential navigation with keyboard shortcuts. Add a new Rust command to resolve direct stream URLs from source URLs via yt-dlp. Create a new `UrlPlayerDialog` for streaming remote URLs. Add a "disable token" toggle in the LAN settings section.

**Tech Stack:** React 19, TypeScript 5.8, TanStack Router, Tauri v2 IPC, Rust (tokio, yt-dlp CLI), Tailwind CSS v4, Radix UI primitives.

---

## Global Constraints

- Package manager: Bun (>=1.3.0) — do not use npm/yarn/pnpm
- Shell: PowerShell 7+ compatible commands
- TypeScript strict mode enabled
- Run `bun run lint`, `bun run format:check`, and `cd src-tauri && cargo test` before declaring work complete
- Do not create git commits unless explicitly asked
- Do not edit `Plan.md`

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `src/components/ScenePlayerDialog.tsx` | Modify | Add scene list navigation, keyboard shortcuts, prev/next buttons |
| `src/components/SceneCard.tsx` | Modify | Add Watch button alongside Download |
| `src/components/UrlPlayerDialog.tsx` | Create | New dialog for streaming remote URLs via resolved direct video URLs |
| `src/routes/library/scenes/index.tsx` | Modify | Pass scene list + current index to ScenePlayerDialog |
| `src/routes/browse/$site/$kind/$slug.tsx` | Modify | Add Watch handler, pass to SceneCard |
| `src/routes/browse/by-url.tsx` | Modify | Add Watch handler, pass to SceneCard |
| `src-tauri/src/commands.rs` | Modify | Add `resolve_stream_url` IPC command |
| `src-tauri/src/lib.rs` | Modify | Register `resolve_stream_url` in invoke handler |
| `src/lib/api/client.ts` | Modify | Add `resolveStreamUrl` method |
| `src/lib/types.ts` | Modify | (no changes needed — MediaItem already sufficient) |
| `src/routes/settings/index.tsx` | Modify | Add "Disable token" toggle in LAN section |
| `src-tauri/src/models.rs` | Modify | Add `lan_auth_enabled` field to AppSettings |
| `src-tauri/src/state.rs` | Modify | Respect `lan_auth_enabled` when starting LAN server |
| `src-tauri/src/server/mod.rs` | Modify | Check `lan_auth_enabled` in auth middleware |

---

## Task 1: Library Player — Scene List Navigation with Arrow Keys

**Files:**
- Modify: `src/components/ScenePlayerDialog.tsx`
- Modify: `src/routes/library/scenes/index.tsx`

**Interfaces:**
- Consumes: `Scene[]` list and current scene index from library page
- Produces: Updated `ScenePlayerDialog` with `scenes` and `currentIndex` props, keyboard navigation, prev/next buttons

### Step 1: Update ScenePlayerDialog props to accept scene list

In `src/components/ScenePlayerDialog.tsx`, update the `ScenePlayerDialogProps` interface:

```typescript
interface ScenePlayerDialogProps {
  scene: Scene | null;
  scenes?: Scene[];        // Full list for navigation
  currentIndex?: number;   // Current index in the list
  open: boolean;
  onClose: () => void;
  onEdit?: (scene: Scene) => void;
  onNavigate?: (scene: Scene, index: number) => void;  // Called when navigating to next/prev
}
```

### Step 2: Add keyboard event handler for arrow keys

Add a `useEffect` in `ScenePlayerDialog` that listens for `ArrowLeft` and `ArrowRight` keydown events when the dialog is open. ArrowLeft calls the previous scene handler, ArrowRight calls the next scene handler. Only active when `scenes` array is provided and has more than 1 item.

```typescript
useEffect(() => {
  if (!open) return;
  function handleKey(e: KeyboardEvent) {
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      if (scenes && currentIndex != null && currentIndex > 0) {
        onNavigate?.(scenes[currentIndex - 1], currentIndex - 1);
      }
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      if (scenes && currentIndex != null && currentIndex < scenes.length - 1) {
        onNavigate?.(scenes[currentIndex + 1], currentIndex + 1);
      }
    }
  }
  window.addEventListener("keydown", handleKey);
  return () => window.removeEventListener("keydown", handleKey);
}, [open, scenes, currentIndex, onNavigate]);
```

### Step 3: Add prev/next navigation buttons to player UI

Inside the `ScenePlayerBody` component, below the video element and above the metadata, add a navigation bar when `scenes` has multiple items:

```tsx
{scenes && scenes.length > 1 && currentIndex != null && (
  <div className="mt-3 flex items-center justify-between">
    <Button
      variant="outline"
      size="sm"
      disabled={currentIndex <= 0}
      onClick={() => onNavigate?.(scenes[currentIndex - 1], currentIndex - 1)}
    >
      ← Previous
    </Button>
    <span className="text-xs text-[var(--color-muted-foreground)]">
      {currentIndex + 1} / {scenes.length}
    </span>
    <Button
      variant="outline"
      size="sm"
      disabled={currentIndex >= scenes.length - 1}
      onClick={() => onNavigate?.(scenes[currentIndex + 1], currentIndex + 1)}
    >
      Next →
    </Button>
  </div>
)}
```

### Step 4: Update ScenePlayerDialog to pass new props through

The outer `ScenePlayerDialog` component must pass `scenes`, `currentIndex`, and `onNavigate` down to `ScenePlayerBody`. Add these to the `ScenePlayerBody` call:

```tsx
<ScenePlayerBody
  key={scene.id}
  scene={scene}
  scenes={scenes}
  currentIndex={currentIndex}
  onClose={onClose}
  onEdit={onEdit}
  onNavigate={onNavigate}
/>
```

### Step 5: Update library scenes page to pass scene list

In `src/routes/library/scenes/index.tsx`, add state for the current player index and pass the scene list to the dialog:

```tsx
const [playerIndex, setPlayerIndex] = useState<number>(0);

// When opening player from card click, set the index:
onClick={() => {
  if (selectionMode) return;
  if (longPressTriggered.current) { longPressTriggered.current = false; return; }
  if (isVideoScene(scene)) {
    setPlayerIndex(scenes.indexOf(scene));
    setPlayerScene(scene);
  } else {
    setDetailsScene(scene);
  }
}}
```

Update the `ScenePlayerDialog` usage:

```tsx
<ScenePlayerDialog
  scene={playerScene}
  scenes={scenes.filter(isVideoScene)}
  currentIndex={playerIndex}
  open={playerScene !== null}
  onClose={() => setPlayerScene(null)}
  onEdit={(s) => { setPlayerScene(null); setEditScene(s); }}
  onNavigate={(s, i) => { setPlayerScene(s); setPlayerIndex(i); }}
/>
```

Note: The `scenes` passed should be filtered to only video scenes (since non-video scenes open `SceneDetailsDialog`), and `playerIndex` should be relative to this filtered list. We need to compute a mapping:

```tsx
const videoScenes = scenes.filter(isVideoScene);

// When clicking a scene card:
const idx = videoScenes.findIndex(v => v.id === scene.id);
if (idx >= 0) {
  setPlayerIndex(idx);
  setPlayerScene(scene);
}
```

### Step 6: Verify

Run: `bun run lint && bun run format:check`
Expected: No errors

---

## Task 2: Browse SceneCard — Add Watch Button

**Files:**
- Modify: `src/components/SceneCard.tsx`
- Create: `src/components/UrlPlayerDialog.tsx`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/api/client.ts`
- Modify: `src/routes/browse/$site/$kind/$slug.tsx`
- Modify: `src/routes/browse/by-url.tsx`

**Interfaces:**
- Consumes: `MediaItem` from browse results
- Produces: `resolveStreamUrl` API method, `UrlPlayerDialog` component, Watch button on SceneCard

### Step 1: Add Rust `resolve_stream_url` command

In `src-tauri/src/commands.rs`, add a new command that uses yt-dlp to resolve a direct streamable URL:

```rust
#[tauri::command]
pub async fn resolve_stream_url(state: State<'_, AppState>, url: String) -> Result<String, String> {
    let settings = state.get_settings().map_err(|e| e.to_string())?;
    let yt_dlp_path = state.yt_dlp_path();

    let output = tokio::process::Command::new(&yt_dlp_path)
        .args(["--get-url", "--no-warnings", &url])
        .output()
        .await
        .map_err(|e| format!("Failed to run yt-dlp: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp failed: {stderr}"));
    }

    let stream_url = String::from_utf8(output.stdout)
        .map_err(|e| format!("Invalid yt-dlp output: {e}"))?
        .trim()
        .to_string();

    if stream_url.is_empty() {
        return Err("No stream URL resolved".to_string());
    }

    Ok(stream_url)
}
```

### Step 2: Register command in lib.rs

In `src-tauri/src/lib.rs`, add `commands::resolve_stream_url` to the invoke handler list:

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    commands::resolve_stream_url,
])
```

### Step 3: Add `resolveStreamUrl` to frontend API client

In `src/lib/api/client.ts`, add the method:

```typescript
async resolveStreamUrl(url: string): Promise<string> {
  return localOrRemote(
    "resolve_stream_url",
    { url },
    `/api/media/stream-url`,
    {
      method: "POST",
      body: JSON.stringify({ url }),
    },
  );
},
```

### Step 4: Add LAN server endpoint for stream URL resolution

In `src-tauri/src/server/mod.rs`, add a POST endpoint `/api/media/stream-url` that calls the same yt-dlp logic. This allows mobile clients to resolve stream URLs via the LAN server.

Add a new handler function:

```rust
async fn resolve_stream_url(
    State(state): State<ApiState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let url = body.get("url")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;

    // Use same yt-dlp resolution logic as the IPC command
    let yt_dlp_path = state.app.yt_dlp_path();
    let output = tokio::process::Command::new(&yt_dlp_path)
        .args(["--get-url", "--no-warnings", url])
        .output()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !output.status.success() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let stream_url = String::from_utf8(output.stdout)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .trim()
        .to_string();

    Ok(Json(serde_json::json!({ "stream_url": stream_url })))
}
```

Register in the router:

```rust
.route("/api/media/stream-url", post(resolve_stream_url))
```

### Step 5: Create UrlPlayerDialog component

Create `src/components/UrlPlayerDialog.tsx`:

```tsx
import { useEffect, useRef, useState } from "react";
import { api } from "@/lib/api/client";
import { Button } from "@/components/ui/button";
import { X, Loader2, AlertCircle } from "lucide-react";
import type { MediaItem } from "@/lib/types";

interface UrlPlayerDialogProps {
  item: MediaItem | null;
  open: boolean;
  onClose: () => void;
}

export function UrlPlayerDialog({ item, open, onClose }: UrlPlayerDialogProps) {
  const [streamUrl, setStreamUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const videoRef = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    if (!open || !item) {
      setStreamUrl(null);
      setError(null);
      return;
    }
    setLoading(true);
    setError(null);
    void api
      .resolveStreamUrl(item.url)
      .then((url) => {
        setStreamUrl(url);
      })
      .catch((e) => {
        setError(e instanceof Error ? e.message : "Failed to resolve stream URL");
      })
      .finally(() => setLoading(false));
  }, [open, item]);

  if (!open || !item) return null;

  return (
    <div className="fixed inset-0 z-[100] flex items-end justify-center bg-black/80 p-0 sm:items-center sm:p-4">
      <div
        role="dialog"
        aria-modal="true"
        className="flex max-h-[92dvh] w-full max-w-4xl flex-col overflow-hidden rounded-t-xl border border-[var(--color-border)] bg-[var(--color-card)] shadow-xl sm:rounded-lg"
      >
        <div className="overflow-y-auto overscroll-contain p-4">
          <div className="mb-3 flex items-center justify-between gap-2">
            <h3 className="text-lg font-semibold leading-snug line-clamp-2">{item.title}</h3>
            <button
              type="button"
              onClick={onClose}
              className="rounded p-2 hover:bg-[var(--color-muted)]"
              aria-label="Close"
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          {loading && (
            <div className="flex aspect-video items-center justify-center rounded-md bg-[var(--color-muted)]">
              <Loader2 className="h-8 w-8 animate-spin text-[var(--color-muted-foreground)]" />
            </div>
          )}

          {error && (
            <div className="flex aspect-video flex-col items-center justify-center gap-2 rounded-md bg-[var(--color-muted)]">
              <AlertCircle className="h-8 w-8 text-red-400" />
              <p className="text-sm text-[var(--color-muted-foreground)]">{error}</p>
              <Button variant="outline" size="sm" onClick={onClose}>
                Close
              </Button>
            </div>
          )}

          {streamUrl && (
            <video
              ref={videoRef}
              key={streamUrl}
              src={streamUrl}
              controls
              playsInline
              autoPlay
              className="aspect-video w-full rounded-md bg-black"
              onError={() => setError("Playback failed — the stream URL may have expired.")}
            />
          )}

          <dl className="mt-4 space-y-2 text-sm">
            {item.performers.length > 0 && (
              <div>
                <dt className="text-xs text-[var(--color-muted-foreground)]">Performers</dt>
                <dd>{item.performers.join(", ")}</dd>
              </div>
            )}
            {item.channel && (
              <div>
                <dt className="text-xs text-[var(--color-muted-foreground)]">Channel</dt>
                <dd>{item.channel}</dd>
              </div>
            )}
            {item.tags.length > 0 && (
              <div>
                <dt className="text-xs text-[var(--color-muted-foreground)]">Tags</dt>
                <dd className="mt-1 flex flex-wrap gap-1">
                  {item.tags.map((tag) => (
                    <span
                      key={tag}
                      className="rounded bg-[var(--color-secondary)] px-1.5 py-0.5 text-xs"
                    >
                      {tag}
                    </span>
                  ))}
                </dd>
              </div>
            )}
            {item.description && (
              <div>
                <dt className="text-xs text-[var(--color-muted-foreground)]">Description</dt>
                <dd className="text-xs text-[var(--color-muted-foreground)] line-clamp-4">
                  {item.description}
                </dd>
              </div>
            )}
          </dl>

          <div className="mt-4 flex justify-end">
            <Button variant="outline" onClick={onClose} className="min-h-10 min-w-[5.5rem]">
              Close
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
```

### Step 6: Update SceneCard to include Watch button

In `src/components/SceneCard.tsx`, add an `onWatch` prop and a Play button:

```tsx
import type { MediaItem } from "@/lib/types";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Download, Clock, Info, Play } from "lucide-react";

interface SceneCardProps {
  item: MediaItem;
  onDownload?: (item: MediaItem) => void;
  onInfo?: (item: MediaItem) => void;
  onWatch?: (item: MediaItem) => void;
}

export function SceneCard({ item, onDownload, onInfo, onWatch }: SceneCardProps) {
  // ... existing card rendering ...

  return (
    <Card className="overflow-hidden transition hover:border-[var(--color-primary)]">
      {/* ... thumbnail and content same as before ... */}
      <div className="flex gap-2">
        {onInfo && (
          <Button size="sm" variant="outline" className="flex-1" onClick={() => onInfo(item)}>
            <Info className="h-3.5 w-3.5" />
            Info
          </Button>
        )}
        {onWatch && (
          <Button size="sm" variant="default" className="flex-1" onClick={() => onWatch(item)}>
            <Play className="h-3.5 w-3.5" />
            Watch
          </Button>
        )}
        {onDownload && (
          <Button size="sm" variant="outline" className="flex-1" onClick={() => onDownload(item)}>
            <Download className="h-3.5 w-3.5" />
            Download
          </Button>
        )}
      </div>
    </Card>
  );
}
```

### Step 7: Wire up Watch in browse detail page

In `src/routes/browse/$site/$kind/$slug.tsx`, add the watch handler and UrlPlayerDialog:

```tsx
import { UrlPlayerDialog } from "@/components/UrlPlayerDialog";

// Add state:
const [watchItem, setWatchItem] = useState<MediaItem | null>(null);

// Update SceneCard usage:
<SceneCard
  key={item.id}
  item={item}
  onDownload={(i) => void handleDownload(i)}
  onInfo={setInfoItem}
  onWatch={setWatchItem}
/>

// Add dialog:
<UrlPlayerDialog
  item={watchItem}
  open={watchItem !== null}
  onClose={() => setWatchItem(null)}
/>
```

### Step 8: Wire up Watch in by-url browse page

Same pattern as Step 7, in `src/routes/browse/by-url.tsx`.

### Step 9: Verify

Run: `bun run lint && bun run format:check && cd src-tauri && cargo test`
Expected: No errors

---

## Task 3: LAN Token Disable Option

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/server/mod.rs`
- Modify: `src/routes/settings/index.tsx`
- Modify: `src/lib/types.ts`

**Interfaces:**
- Consumes: AppSettings with new `lan_auth_enabled` field
- Produces: Toggle in LAN settings, middleware checks the flag

### Step 1: Add `lan_auth_enabled` to Rust AppSettings

In `src-tauri/src/models.rs`, add a new field to `AppSettings`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    // ... existing fields ...
    #[serde(default = "default_true")]
    pub lan_auth_enabled: bool,
}

fn default_true() -> bool {
    true
}
```

Update the `Default` impl:

```rust
Self {
    // ... existing fields ...
    lan_auth_enabled: true,
}
```

### Step 2: Update auth middleware to check the flag

In `src-tauri/src/server/mod.rs`, modify `auth_middleware` to also skip auth when `lan_auth_enabled` is false:

```rust
async fn auth_middleware(
    State(state): State<ApiState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    if req.uri().path() == "/api/health" || !req.uri().path().starts_with("/api/") {
        return next.run(req).await;
    }
    // Skip auth if token is empty OR auth is disabled
    if state.token.is_empty() || !state.lan_auth_enabled {
        return next.run(req).await;
    }
    // ... existing token checking logic ...
}
```

Add `lan_auth_enabled` to the `ApiState` struct:

```rust
struct ApiState {
    app: AppState,
    token: String,
    lan_auth_enabled: bool,
    // ...
}
```

Pass it when constructing `ApiState` in `LanServer::start()`.

### Step 3: Update ensure_lan_server to respect the flag

In `src-tauri/src/state.rs`, modify `ensure_lan_server` to skip token generation when auth is disabled:

```rust
pub async fn ensure_lan_server(self: &Arc<Self>, port: u16) -> AppResult<String> {
    // ...
    let mut settings = self.get_settings()?;
    let token = if !settings.lan_auth_enabled {
        String::new()  // No token needed
    } else if open_dev {
        String::new()
    } else {
        match settings.lan_token.clone() {
            Some(t) if !t.is_empty() => t,
            _ => crate::server::generate_token(),
        }
    };
    // ... rest of function ...
}
```

### Step 4: Add TypeScript type mirror

In `src/lib/types.ts`, add `lan_auth_enabled` to `AppSettings`:

```typescript
export interface AppSettings {
  // ... existing fields ...
  lan_auth_enabled?: boolean;
}
```

### Step 5: Add toggle in Settings LAN tab

In `src/routes/settings/index.tsx`, add a "Require authentication" toggle below the Enable LAN switch:

```tsx
{settings.lan_enabled && (
  <div className="flex items-center gap-3">
    <Switch.Root
      checked={settings.lan_auth_enabled !== false}
      onCheckedChange={(checked) => updateSettings({ lan_auth_enabled: checked })}
      className="h-5 w-9 rounded-full bg-[var(--color-secondary)] data-[state=checked]:bg-[var(--color-primary)]"
    >
      <Switch.Thumb className="block h-4 w-4 translate-x-0.5 rounded-full bg-white transition data-[state=checked]:translate-x-[18px]" />
    </Switch.Root>
    <div>
      <span className="text-sm">Require authentication</span>
      <p className="text-xs text-[var(--color-muted-foreground)]">
        When disabled, any device on your LAN can access the API without a token.
      </p>
    </div>
  </div>
)}
```

### Step 6: Verify

Run: `bun run lint && bun run format:check && cd src-tauri && cargo test`
Expected: No errors

---

## Task 4: Integration Verification

### Step 1: Run full lint and format check

```bash
bun run lint && bun run format:check
```

### Step 2: Run Rust tests

```bash
cd src-tauri && cargo test
```

### Step 3: Manual testing checklist

1. **Library player navigation**: Open library → click a video scene → arrow left/right should navigate between video scenes → prev/next buttons should appear and work
2. **Browse Watch button**: Go to a browse page → SceneCard should show Info, Watch, and Download buttons → clicking Watch should open UrlPlayerDialog → resolving stream URL should play the video
3. **LAN token disable**: Settings → LAN → toggle off "Require authentication" → restart LAN server → mobile app should connect without token
4. **Keyboard shortcuts**: In library player, press Left arrow → should go to previous scene; press Right arrow → should go to next scene; pressing when at first/last scene should be a no-op

---

## Risk Notes

- **yt-dlp availability**: The `resolve_stream_url` command requires yt-dlp to be installed as a sidecar. On mobile/standalone mode, this won't work. The `UrlPlayerDialog` should show a clear error if yt-dlp is unavailable.
- **Stream URL expiry**: Some sites generate expiring URLs. If playback fails after a few minutes, the user should close and re-resolve.
- **Token security**: Disabling the LAN token makes the API accessible to anyone on the LAN. The UI should warn about this.
- **Scene list filtering**: When passing scenes to the player for navigation, only video scenes (mp4/m4v/webm) should be included, since non-video scenes open a different dialog.
