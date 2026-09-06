# ArcHive — Feature Suggestions

> Generated from codebase analysis. Features are grouped by priority and categorized by area.
> Legend: ✅ Done · 🔵 Partial · ⚪ Not started

---

## High Priority — Core Experience Gaps

### 1. Smart Auto-Tagging from Metadata

**Status:** 🔵 Partial  
**Area:** Library / Auto-Tagger  
**Currently:** Filename regex rules (`AppSettings.auto_tag_rules`, default `(?<performer>[a-zA-Z0-9_]+)-\d+`) + performers/tags passthrough from `DownloadPlan` → `download_jobs.metadata` JSON → scene rows on import (`src-tauri/src/library/auto_tag.rs`, `import.rs`).  
**Suggestion:** Expand auto-tagger to extract performers, studios, and tags from yt-dlp `--dump-json` output, site-scraped metadata, and filename conventions. Apply heuristics like:

- Match performer aliases against known names (fuzzy match).
- Infer studios from source URL domain (e.g., `pornhub.com/studio/...` → studio record).
- Tag scenes automatically from yt-dlp `categories` and `tags` fields.
- Show a "suggested tags" UI in the scene edit dialog for user confirmation.

### 2. Watchlists / Favorites System

**Status:** ⚪ Not started  
**Area:** Library / Browse  
**Currently:** `Performer.favorite` bool exists (`performers.favorite`, managed in performers page) but no saved searches, subscriptions, or new-match detection.  
**Suggestion:** Allow users to star/bookmark performers and save search queries as watchlists. When a new scene appears on a site matching a saved search, show it in a "New Matches" dashboard. Wire into the Browse page so users can subscribe to tag/model pages and get notified of new content.

### 3. Bulk Download Queue Management

**Status:** 🔵 Partial  
**Area:** Downloads  
**Currently:** Queue has pause/resume/cancel/retry (`commands.rs`, `downloads/manager.rs`), retry with backoff (`retry_count`, `last_retry_at`, `download_max_retries`, `download_retry_delay_seconds`), bulk URL import with browse expansion (`BulkImportPanel.tsx`, `downloads/bulk.rs`, `queue_bulk_import`).  
**Suggestion:** Add:

- **Priority reordering** — drag-and-drop reorder in the download queue.
- **Concurrent download limit** — configurable max parallel downloads (currently seems unbounded).
- **Bandwidth throttling** — per-download or global speed limit (yt-dlp `--limit-rate`).
- **Queue auto-start** — toggle to auto-start downloads when added (currently manual).
- **Retry failed with backoff** — ✅ done, expose max attempts/delay in Settings UI.

### 4. Scene Watch History / Playback Position Memory

**Status:** ⚪ Not started  
**Area:** Library / Player  
**Currently:** No `watch_progress` column; `ScenePlayerDialog` plays but does not persist position. `Scene` has `rating: Option<u8>` column unused.  
**Suggestion:** Track playback position per scene (resume where you left off). Store in SQLite with a `watch_progress` column (position in seconds + timestamp). Display a progress bar overlay on SceneCard thumbnails for partially watched scenes. Mark scenes as "watched" after 90% completion. See also #26 for full history spec.

### 5. Improved Duplicate Detection UI

**Status:** 🔵 Partial  
**Area:** Library / Duplicates  
**Currently:** `DuplicateGroupCard` + `DuplicateMergeCard` show groups; `find_duplicates` (pHash + OSHash) and `merge_duplicates(keep_id, remove_ids, delete_files)` work; `/duplicates` route + `EmptyState` exist.  
**Suggestion:**

- Side-by-side video preview in duplicate resolution dialog.
- Show file size, resolution, duration, and quality info for each duplicate.
- "Keep best quality" auto-resolution with one-click merge.
- Batch merge/resolve across multiple duplicate groups.

---

## Medium Priority — Feature Enrichment

### 6. Scene Collections / Playlists

**Status:** ⚪ Not started  
**Area:** Library  
**Currently:** No collections table or `/library/collections` route.  
**Suggestion:** Let users create named collections (e.g., "Favorites", "Watch Later", "Custom Compilation"). Scenes can belong to multiple collections. Collections are browsable as a dedicated route (`/library/collections/`). Support reordering within a collection. Useful for curating content from different sites. See #27 for smart-collection extension.

### 7. Studio / Channel Tracking

**Status:** 🔵 Partial  
**Area:** Library / Browse  
**Currently:** `Studio` model exists in Rust with `#[allow(dead_code)]`; `Scene.studio_id/studio_name`, `channel` columns exist and are populated from `DownloadPlan.channel` on import. No studio pages or browser UI.  
**Suggestion:** Fully wire up studios:

- Studio detail page showing all scenes from that studio.
- Auto-link scenes to studios from source URLs.
- Studio browser in Browse section (group channels by studio).
- Studio-level stats (scene count, average rating).

### 8. Export / Import Library Metadata

**Status:** ⚪ Not started  
**Area:** Library  
**Currently:** Only `export_performers` (full performer list, no scenes/tags) exists. No scene/JSON/CSV export, no import, no Stash compat.  
**Suggestion:** Export the full library database (scenes, performers, tags, studios) as JSON or CSV. Import from backup. Useful for migration between machines or backup before destructive operations. Could also support Stash-compatible JSON import for users migrating from Stash. See #32 for full backup/restore.

### 9. Custom Naming Templates with Preview

**Status:** 🔵 Partial  
**Area:** Settings / Library  
**Currently:** `naming_template` is a plain string (`{performer}/{title}.{ext}`); `downloads/naming.rs` renders it. No preview or validation UI.  
**Suggestion:**

- Visual template builder with available variables: `{performer}`, `{title}`, `{site}`, `{date}`, `{tags}`, `{studio}`, `{duration}`.
- Live preview showing sample output for a selected scene.
- Template validation (detect illegal path characters per OS).
- Per-site template overrides (e.g., different naming for Pornhub vs YouTube).

### 10. Keyboard Shortcuts & Command Palette

**Status:** 🔵 Partial  
**Area:** Frontend / UX  
**Currently:** `CommandPalette.tsx` (lists registered shortcuts, `Ctrl+K` open via `shortcut:cmd-palette` event), `lib/shortcuts/registry.ts` + `defaults.ts`, `useKeyboardShortcuts.ts` wired in `AppShell`, `ShortcutHelp` dialog + `ShortcutBadge` component exist.  
**Suggestion:** Extend the existing system:

- `Ctrl+K` or `Ctrl+Shift+P` opens a command palette (à la VS Code) — ✅ done, add route navigation + actions (scan library, open settings, new download).
- Configurable shortcut map in Settings (custom keybindings, enable/disable).
- Quick-jump to scenes, performers, tags by name from the palette (fuzzy search over library, not just commands).

### 11. Notification System

**Status:** ⚪ Not started  
**Area:** UX / Downloads  
**Currently:** Desktop tray exists (`desktop/tray.rs`, close-to-tray/minimize-to-tray, `Ctrl+Shift+A` hotkey) but no download/scan/duplicate notifications; no toast system.  
**Suggestion:** System tray notifications (desktop) or in-app toast notifications for:

- Download completed / failed.
- Library scan finished.
- New duplicates detected.
- LAN connection status changes.
- Use Tauri's notification API for native desktop notifications.

### 12. Advanced Search with Filters

**Status:** 🔵 Partial  
**Area:** Library  
**Currently:** `SceneFilter {missing_thumb, missing_duration, min_duration, max_duration, hash_named, performer_names, tag_names}` + `list_scenes_with_filter`, `SceneSort {Newest, Name, Downloaded}`, FTS5 title search, filter chips in `library/scenes/index.tsx` — ✅ duration + missing-thumb/duration filters done.  
**Suggestion:**

- Filter by date range, rating, file size (duration ✅ done).
- Filter by performer count, tag count.
- Boolean tag combinations (AND/OR).
- Regex search mode.
- Save search filters as presets (feeds into #27/#28).

---

## Lower Priority — Polish & Power Features

### 13. Auto-Organize Files on Download

**Status:** 🔵 Partial  
**Area:** Downloads / Library  
**Currently:** `naming_template` controls output path; `update_scene(rename_file)` can rename on edit. No post-download move/copy pass or site-grouping.  
**Suggestion:** Post-download auto-organize:

- Move files into `{performer}/{title}.{ext}` structure automatically.
- Option to copy vs move (keep original in downloads folder).
- Auto-rename existing files when metadata changes (e.g., performer name corrected).
- Organize by site as a top-level grouping option.

### 14. Watch Folder / Auto-Import

**Status:** ⚪ Not started  
**Area:** Library  
**Currently:** Manual `scan_library` only (with `library:scan-progress` events + post-scan `generate_missing_thumbs`). No filesystem watcher.  
**Suggestion:** Monitor a configurable directory for new video/image files. When new files appear, automatically import them into the library:

- Probe metadata (duration, resolution, thumbnail).
- Attempt auto-tagging from filename.
- Show import progress in a dedicated panel.
- Configurable file extensions to watch.

### 15. Scene Notes / Annotations

**Status:** ✅ Done  
**Area:** Library  
**Currently:** `scenes.notes TEXT` (`MIGRATION_006`), `Scene.notes`, `UpdateSceneRequest.notes`, `state.update_scene(..., notes)`, editable in scene edit/details UI.  
**Suggestion (remaining):** Make notes searchable via FTS5 (currently FTS indexes title only — extend `scenes_fts` to include notes) and add timestamp-jump annotations (`[01:23] note` click-to-seek in player).

### 16. Per-Scene Custom Thumbnail

**Status:** 🔵 Partial  
**Area:** Library  
**Currently:** Thumbnails auto-generated (remote URL → probe frame via `generate_missing_thumbs`, `probe_scene_metadata`); `clear_scene_thumb` + orphan sidecar cleaner (`list_orphan_sidecars`) exist. No custom upload.  
**Suggestion:** Allow users to:

- Upload a custom thumbnail image for a scene.
- Scrub through the video to select a specific frame.
- Set thumbnail from a URL.

### 17. Theme Customization

**Status:** 🔵 Partial  
**Area:** Frontend / Settings  
**Currently:** `AppTheme {Dark, Light, System}`, `useTheme.ts`, header toggle in `AppShell` + full selector in Settings; dark-first oklch tokens.  
**Suggestion:**

- Light/dark toggle in Settings — ✅ done (also header toggle ✅).
- Custom accent color picker.
- Font size / density settings (compact, comfortable, spacious).
- Custom CSS injection for power users.

### 18. Plugin Marketplace / Registry

**Status:** 🔵 Partial  
**Area:** Plugins  
**Currently:** Bun+TS plugin system works (`plugins/`, `plugins:generate`, `lib/plugins/loader.ts`, `registry.generated.ts`, `docs/plugins.md`, auto-run on `predev`/`prebuild`). Install is manual `git clone`.  
**Suggestion:**

- In-app plugin browser that lists available community plugins from a JSON registry.
- One-click install (auto-clone + `plugins:generate`).
- Plugin version checking and update notifications.
- Plugin enable/disable toggle without deleting.

### 19. Scheduled Downloads

**Status:** ⚪ Not started  
**Area:** Downloads  
**Currently:** No scheduler; queue starts immediately or paused. No quiet-hours concept.  
**Suggestion:** Queue downloads to start at a specific time or on a schedule. Useful for:

- Off-peak bandwidth usage.
- Recurring downloads (e.g., daily channel check — pairs with #28 watchlists).
- Pause during certain hours (e.g., no downloads during work hours).

### 20. Mobile: Offline Queue Sync

**Status:** ⚪ Not started  
**Area:** Mobile / LAN  
**Currently:** Mobile standalone mode is limited to `resolve_standalone` (YouTube + direct URLs); remote-LAN is the default engine on mobile. No offline outbox.  
**Suggestion:** When the mobile client reconnects to LAN after being offline, sync any queued download requests. Allow users to queue URLs while offline (standalone mode stores them locally), then batch-send to the desktop when reconnected. See #36 for concrete design.

---

## Infrastructure / Technical Improvements

### 21. Database Migration Tooling

**Status:** 🔵 Partial  
**Area:** Rust / DB  
**Currently:** `db/migrations.rs` (`MIGRATION_001`–`007`) with `column_exists` guard in `Database::new()`. No CLI runner, rollback, or integrity check.  
**Suggestion:** Add a `db migrate` CLI command (via Tauri CLI or standalone) to:

- Run pending migrations without starting the full app.
- Roll back migrations.
- Validate migration integrity.

### 22. Automated Backup

**Status:** ⚪ Not started  
**Area:** Library  
**Currently:** No scheduled backup or restore UI. Manual SQLite file copy is the only path.  
**Suggestion:** Scheduled SQLite backup (copy the `.db` file to a backup directory). Configurable backup interval and retention. One-click restore from backup in Settings. See #32 for full backup (DB + thumbs + settings).

### 23. Performance Monitoring Dashboard

**Status:** 🔵 Partial  
**Area:** Settings  
**Currently:** `get_library_stats` → `LibraryStats {scene_count, performer_count, tag_count, total_size_bytes, free_space_bytes}` shown on `/library` index and as nav badge in `AppShell`. No download/LAN/DB metrics.  
**Suggestion:** Show in Settings:

- Library stats — ✅ done (scene count, total size, free space).
- Duplicate count — backend `find_duplicates` exists, surface the count.
- Download queue stats (active, pending, completed, failed).
- LAN connection stats (active clients, bandwidth usage).
- Database size and query performance metrics.

### 24. Content Rating / Parental Controls

**Status:** 🔵 Partial  
**Area:** Library / Settings  
**Currently:** `scenes.rating INTEGER` column + `Scene.rating: Option<u8>` exist but are always `None` (never read/written in UI or filters).  
**Suggestion:** Add a content rating system (e.g., SFW / NSFW / Explicit). Allow users to:

- Tag scenes with a rating (wire up existing column + `update_scene`).
- Filter by rating in browse and library views.
- Set a default rating filter per session.
- Lock certain ratings with a PIN (parental controls).

### 25. REST API Pagination & Filtering

**Status:** ⚪ Not started  
**Area:** LAN Server  
**Currently:** Axum LAN server serves SPA + `GET /api/scenes|performers|tags|downloads|sites`, scene media streaming with Range support, `/files` browser; bearer-token auth. No `page/per_page/sort/filter` params (server `list_scenes` ignores them).  
**Suggestion:** Add query parameters to LAN API endpoints:

- `?page=1&per_page=50` for paginated responses.
- `?sort=newest&order=desc` for sorting.
- `?performer=X&tag=Y` for filtering.
- `?search=query` for search.
- This would make the LAN API more useful for third-party integrations. See #37 for auth scopes.

---

## New Suggestions (2026-09 Research)

### 26. Playback Resume + Watch History

**Status:** ✅ Done  
**Area:** Library / Player  
**Currently:** `watch_history` table (`MIGRATION_009`) with `record_watch_progress` (5s-throttled `timeupdate` + pause/close flush), `get/list_watch_progress`, `mark_watched` commands and LAN routes; `ScenePlayerDialog` shows a resume banner; `SceneCard` shows a progress bar + Watched chip (grid + list, Home rail + library); context-menu and bulk-bar mark watched/unwatched; watched latch at `AppSettings.watched_threshold` (default 0.9) with a slider in Settings → Library; "Hide watched" chip in the scenes filter bar (`SceneFilter.hide_watched`, SQL `NOT EXISTS`).

### 27. Smart Collections (Saved Filters as Auto-Playlists)

**Status:** ⚪ Not started  
**Area:** Library  
**Currently:** No collections; `SceneFilter` exists but cannot be saved.  
**Suggestion:**

- Manual collections (join table `collection_scenes`) + smart collections (stored `SceneFilter` re-evaluated on open).
- `/library/collections/` route with cover mosaic, drag reorder, add-from-context-menu ("Add to collection").
- Export collection as M3U / share over LAN.

### 28. Saved Searches + New-Match Notifications

**Status:** ⚪ Not started  
**Area:** Browse / Library  
**Currently:** Browse pages (`/$site/$kind/$slug`, Pornhub categories, trending sites setting) are ephemeral; no persistence.  
**Suggestion:**

- Save any browse/search (site + kind + slug + orientation) as a named watch; background poller checks on interval; "New Matches" badge + optional tray toast (#11).
- Pairs with #19 scheduled downloads (auto-queue new matches) and #26 history (hide already-watched).

### 29. Performer Detail Pages + Tag Hierarchy Manager

**Status:** ⚪ Not started  
**Area:** Library  
**Currently:** Performer/tag list pages exist with scene counts; `ensure_performer`, favorite toggle, aliases stored but no detail view, merge, or hierarchy UI (`Tag.parent_id` unused in UI).  
**Suggestion:**

- `/library/performers/$id` and `/library/tags/$id` pages: bio/aliases editor, image wall, scene grid, merge-duplicate-performers tool, alias fuzzy-match assist (#1).
- Tag tree editor (drag to re-parent), bulk retag, tag synonyms.

### 30. Live Recorder + Schedule (Chaturbate / Stripchat)

**Status:** ⚪ Not started  
**Area:** Live / Downloads  
**Currently:** Live browse + `resolve_livestream` (HLS `stream_url` + `embed_url` fallback, `hls.js` playback, `$site/$slug` live routes, HTTP+webview scraping tiers) can watch but not record.  
**Suggestion:**

- "Record live" button captures HLS to library path with `naming_template`; scheduled recording windows per model; auto-record favorited models when online.
- Recording indicator in downloads queue; post-process (thumbnail + duration probe + auto-tag) via existing import pipeline.

### 31. Video Preview Scrub / Storyboard Thumbnails

**Status:** ⚪ Not started  
**Area:** Library / Player  
**Currently:** Single sidecar `{stem}.jpg` thumbnail only; no sprite sheets or scrub. `ffmpeg` sidecar already bundled.  
**Suggestion:**

- `ffmpeg` contact-sheet generation (e.g., 5x5 grid) on import/probe; hover-scrub preview on `SceneCard` (CSS sprite offset).
- "Select frame as thumbnail" scrubber in player feeds into #16 custom thumbnail.

### 32. Full Library Backup & Restore (DB + Thumbs + Settings)

**Status:** ⚪ Not started  
**Area:** Library / Settings  
**Currently:** No backup UI (see #22 for DB-only scheduled copy). Thumbs are sidecar `.jpg` files; settings in SQLite `app_settings`.  
**Suggestion:**

- One-click `.archhive-backup.zip` (SQLite + `app_settings` + sidecar thumbs + plugin list); retention policy + scheduled job; restore wizard with path remap (Windows ↔ Android) and dry-run validation.
- Pre-restore safety snapshot; backup manifest with version check against `MIGRATION_*`.

### 33. In-App Log Viewer + Diagnostics Export

**Status:** ⚪ Not started  
**Area:** Settings / Support  
**Currently:** Errors surface as toasts/dialog strings; `ffmpeg_status`, download `error` column exist but no central log view. `docs/troubleshooting-android.md` is manual.  
**Suggestion:**

- Settings → Diagnostics: rolling log (download errors, scan skips, LAN auth failures, yt-dlp stderr tail), yt-dlp/gallery-dl/ffmpeg version cards, "Copy diagnostics" / "Export .zip" for bug reports.
- Per-download "View log" drawer (stderr excerpt) to cut triage time.

### 34. Binary Update Manager (yt-dlp / gallery-dl / ffmpeg)

**Status:** ⚪ Not started  
**Area:** Downloads / Settings  
**Currently:** `setup:binaries` script + `src-tauri/binaries/` sidecars; `ffmpeg_status` probes availability; no version check or in-app update.  
**Suggestion:**

- Version check (yt-dlp `--version` vs GitHub releases) on launch/weekly; one-click update (re-run setup flow elevated if needed); per-tool status dots + changelog link.
- Pin/rollback to last-known-good binary; Android note (no sidecars — desktop only).

### 35. Transcode / Optimize for Mobile & LAN

**Status:** ⚪ Not started  
**Area:** Media / LAN  
**Currently:** Direct streaming with Range (`server/streaming.rs`, 256KB chunks, MIME map) assumes client can play source; `download_quality` + `prefer_mp4` only affect fetch, not existing files.  
**Suggestion:**

- "Optimize" action per scene / bulk: `ffmpeg` re-encode to H.264+AAC MP4 (720p/480p presets), keep original optionally, update `file_size`/`duration`/thumb.
- Low-bandwidth LAN mode: on-the-fly or cached transcode for `/api/scenes/{id}/media` when client hints `?quality=low`.

### 36. Offline Outbox for Mobile (Concrete Sync Design)

**Status:** ⚪ Not started  
**Area:** Mobile / LAN  
**Currently:** See #20; mobile defaults to `RemoteLan`, `lan-discovery` + `lanBootstrap` handle pairing, no queued intents.  
**Suggestion:**

- `localStorage` outbox (`url`, `adapter?`, `createdAt`, `status`) in `api/client.ts` remote mode; "Queued offline (3)" badge; on reconnect flush via `queue_downloads` batch + per-item receipt; conflict display for already-queued URLs.
- Share-target intent: Android share sheet → outbox in one tap.

### 37. LAN API Auth Scopes + Read-Only Tokens

**Status:** ⚪ Not started  
**Area:** LAN Server / Security  
**Currently:** Single bearer token (`lan_token`, `lan_auth_enabled`); all routes except `/api/health` require it; mDNS `_archhive._tcp` advertises.  
**Suggestion:**

- Scoped tokens: `read` (browse/library/stream) vs `admin` (queue/delete/settings); per-device labels + revoke list in Settings → LAN.
- Rate-limit download-queue endpoints; audit log of remote actions. Builds on #25 pagination work for third-party clients.

### 38. Subtitle / Caption Fetch + Display

**Status:** ⚪ Not started  
**Area:** Media / Downloads  
**Currently:** No subtitle handling; player dialogs have no track selection; `DownloadPlan` has no subtitle fields.  
**Suggestion:**

- yt-dlp `--writesubs --writeautosubs` fetch on demand; sidecar `.vtt` next to video; player `<track>` picker + language preference in Settings.
- Auto-embed vs sidecar toggle; LAN streaming serves `.vtt` alongside media.

### 39. Bulk File Operations with Undo

**Status:** ⚪ Not started  
**Area:** Library / Files  
**Currently:** `SceneBulkEditBar` (tag/performer batch), `delete_scene(delete_files)`, orphan sidecar list + delete, `/files` browser exist — but no move/rename bulk, no trash/undo.  
**Suggestion:**

- Multi-select move/rename (apply `naming_template` to selection), trash-with-restore (30-day) instead of hard delete, undo toast for last bulk op.
- Surface orphan-sidecar cleaner in Files UI with "select all → delete" + size totals.

### 40. First-Run Wizard + Sample Library

**Status:** ⚪ Not started  
**Area:** Onboarding / UX  
**Currently:** No wizard; engine mode/LAN/library path/cookies all configured deep in Settings; `docs/*` carry the onboarding burden.  
**Suggestion:**

- 5-step wizard: library path → engine mode (Local/Remote/Standalone) → LAN pairing (QR/discovery) → cookie import (`docs/cookie-import.md` flow) → binary check (`ffmpeg_status`).
- "Load demo scene" for empty-state testing; post-wizard checklist with deep links; changelog dialog (#Q20) on first launch per version.

---

## Quick Wins (< 1 day each)

| #   | Feature                       | Description                                                                                                                                      | Status                                                                                 |
| --- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------- |
| Q1  | **Scene count badge**         | Show total scene count on the Library nav item.                                                                                                  | ✅ Done (`AppShell` `scene_count` + `/library` stats)                                  |
| Q2  | **Last downloaded sort**      | Add "Recently Downloaded" sort option to scenes.                                                                                                 | ✅ Done (`SceneSort::Downloaded`)                                                      |
| Q3  | **File size display**         | Show file size on SceneCard in grid/list view.                                                                                                   | ✅ Done (`Scene.file_size`, `SceneCard` + details dialogs)                             |
| Q4  | **Performer image upload**    | Allow setting performer profile images manually.                                                                                                 | ✅ Done (`set_performer_image`)                                                        |
| Q5  | **Dark/light theme toggle**   | Simple toggle button in the header bar.                                                                                                          | ✅ Done (`AppShell` toggle + Settings selector, `AppTheme`)                            |
| Q6  | **Keyboard shortcut hints**   | Show shortcut keys in tooltips and menus.                                                                                                        | 🔵 Partial (`ShortcutBadge` + `ShortcutHelp`; tooltips/menu coverage incomplete)       |
| Q7  | **Export performer list**     | Simple CSV/JSON export of all performers.                                                                                                        | ✅ Done (`export_performers`; CSV format still open)                                   |
| Q8  | **Scene duration filter**     | Add min/max duration inputs to the scenes filter bar.                                                                                            | ✅ Done (`SceneFilter.min/max_duration` + chips)                                       |
| Q9  | **Empty state illustrations** | Add friendly illustrations to empty states.                                                                                                      | ✅ Done (`EmptyState` + `ErrorState` + skeletons across routes; custom art still open) |
| Q10 | **Changelog in-app**          | Show recent changes on first launch after update.                                                                                                | ⚪ Not started                                                                         |
| Q11 | **Recently played rail**      | "Continue watching" strip on Home (`lib/stores/recentlyViewed.ts` persisted, recorded in `ScenePlayerDialog`, 12-item rail with Clear).          | ✅ Done                                                                                |
| Q12 | **Copy scene debug JSON**     | Copy button in Scene Details (id, path, hashes, source URL, resolution, size) for bug reports.                                                   | ✅ Done                                                                                |
| Q13 | **Binary version card**       | "Media tools" card in Settings → Library (`binary_versions` command + `/api/system/versions`: yt-dlp / gallery-dl / ffmpeg / ffprobe).           | ✅ Done                                                                                |
| Q14 | **Duplicate count badge**     | Duplicates entry in desktop sidebar with group-count badge (99+ cap) + `Ctrl+7` shortcut.                                                        | ✅ Done                                                                                |
| Q15 | **Performer sort by scenes**  | Sort toggle (Name / Scenes) on performers page.                                                                                                  | ✅ Done                                                                                |
| Q16 | **Resolution badge**          | WxH badge on SceneCard (grid + list) from probed data (`scenes.width/height`, `MIGRATION_008`; backfilled by probe flows + details/player rows). | ✅ Done                                                                                |
| Q17 | **Clear thumbnail cache**     | "Clear thumbnail cache" in Settings → Library (`clear_all_thumbs` command + `DELETE /api/library/thumbs`, confirm + rebuild hint).               | ✅ Done                                                                                |
| Q18 | **`?` opens shortcuts**       | Fixed `?`/`Shift+/` never matching in `shortcuts/registry.ts` (Shift-mask exemption for symbol keys); `ShortcutHelp` now opens.                  | ✅ Done                                                                                |
| Q19 | **Orphan cleanup totals**     | "Orphan sidecars" card in Settings → Library: scan, count + reclaimable bytes, per-item and Delete-all.                                          | ✅ Done                                                                                |
| Q20 | **LAN copy-address**          | Already shipped: "Copy web link" in Settings → LAN copies `http://<lan-ip>:<port>/?token=…`.                                                     | ✅ Done                                                                                |

---

_Last updated: 2026-09-06 (Q11–Q20 implemented)_
