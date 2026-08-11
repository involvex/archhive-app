# ArcHive — Feature Suggestions

> Generated from codebase analysis. Features are grouped by priority and categorized by area.

---

## High Priority — Core Experience Gaps

### 1. Smart Auto-Tagging from Metadata

**Area:** Library / Auto-Tagger  
**Currently:** Filename regex rules + metadata from download jobs.  
**Suggestion:** Expand auto-tagger to extract performers, studios, and tags from yt-dlp `--dump-json` output, site-scraped metadata, and filename conventions. Apply heuristics like:

- Match performer aliases against known names (fuzzy match).
- Infer studios from source URL domain (e.g., `pornhub.com/studio/...` → studio record).
- Tag scenes automatically from yt-dlp `categories` and `tags` fields.
- Show a "suggested tags" UI in the scene edit dialog for user confirmation.

### 2. Watchlists / Favorites System

**Area:** Library / Browse  
**Suggestion:** Allow users to star/bookmark performers and save search queries as watchlists. When a new scene appears on a site matching a saved search, show it in a "New Matches" dashboard. Wire into the Browse page so users can subscribe to tag/model pages and get notified of new content.

### 3. Bulk Download Queue Management

**Area:** Downloads  
**Currently:** Queue has basic pause/resume/cancel.  
**Suggestion:** Add:

- **Priority reordering** — drag-and-drop reorder in the download queue.
- **Concurrent download limit** — configurable max parallel downloads (currently seems unbounded).
- **Bandwidth throttling** — per-download or global speed limit.
- **Queue auto-start** — toggle to auto-start downloads when added (currently manual).
- **Retry failed with backoff** — exponential backoff retry with max attempts configurable.

### 4. Scene Watch History / Playback Position Memory

**Area:** Library / Player  
**Suggestion:** Track playback position per scene (resume where you left off). Store in SQLite with a `watch_progress` column (position in seconds + timestamp). Display a progress bar overlay on SceneCard thumbnails for partially watched scenes. Mark scenes as "watched" after 90% completion.

### 5. Improved Duplicate Detection UI

**Area:** Library / Duplicates  
**Currently:** `DuplicateGroupCard` shows groups.  
**Suggestion:**

- Side-by-side video preview in duplicate resolution dialog.
- Show file size, resolution, duration, and quality info for each duplicate.
- "Keep best quality" auto-resolution with one-click merge.
- Batch merge/resolve across multiple duplicate groups.

---

## Medium Priority — Feature Enrichment

### 6. Scene Collections / Playlists

**Area:** Library  
**Suggestion:** Let users create named collections (e.g., "Favorites", "Watch Later", "Custom Compilation"). Scenes can belong to multiple collections. Collections are browsable as a dedicated route (`/library/collections/`). Support reordering within a collection. Useful for curating content from different sites.

### 7. Studio / Channel Tracking

**Area:** Library / Browse  
**Currently:** `Studio` model exists in Rust with `#[allow(dead_code)]`.  
**Suggestion:** Fully wire up studios:

- Studio detail page showing all scenes from that studio.
- Auto-link scenes to studios from source URLs.
- Studio browser in Browse section (group channels by studio).
- Studio-level stats (scene count, average rating).

### 8. Export / Import Library Metadata

**Area:** Library  
**Suggestion:** Export the full library database (scenes, performers, tags, studios) as JSON or CSV. Import from backup. Useful for migration between machines or backup before destructive operations. Could also support Stash-compatible JSON import for users migrating from Stash.

### 9. Custom Naming Templates with Preview

**Area:** Settings / Library  
**Currently:** `naming_template` is a plain string (`{performer}/{title}.{ext}`).  
**Suggestion:**

- Visual template builder with available variables: `{performer}`, `{title}`, `{site}`, `{date}`, `{tags}`, `{studio}`, `{duration}`.
- Live preview showing sample output for a selected scene.
- Template validation (detect illegal path characters per OS).
- Per-site template overrides (e.g., different naming for Pornhub vs YouTube).

### 10. Keyboard Shortcuts & Command Palette

**Area:** Frontend / UX  
**Suggestion:** Add a global keyboard shortcut system:

- `Ctrl+K` or `Ctrl+Shift+P` opens a command palette (à la VS Code).
- Navigate to any route, trigger actions (scan library, open settings, new download).
- Configurable shortcut map in Settings.
- Quick-jump to scenes, performers, tags by name from the palette.

### 11. Notification System

**Area:** UX / Downloads  
**Suggestion:** System tray notifications (desktop) or in-app toast notifications for:

- Download completed / failed.
- Library scan finished.
- New duplicates detected.
- LAN connection status changes.
- Use Tauri's notification API for native desktop notifications.

### 12. Advanced Search with Filters

**Area:** Library  
**Currently:** Scene search is basic text query + sort.  
**Suggestion:**

- Filter by date range, duration range, rating, file size.
- Filter by performer count, tag count.
- Boolean tag combinations (AND/OR).
- Regex search mode.
- Save search filters as presets.

---

## Lower Priority — Polish & Power Features

### 13. Auto-Organize Files on Download

**Area:** Downloads / Library  
**Currently:** `naming_template` controls output path.  
**Suggestion:** Post-download auto-organize:

- Move files into `{performer}/{title}.{ext}` structure automatically.
- Option to copy vs move (keep original in downloads folder).
- Auto-rename existing files when metadata changes (e.g., performer name corrected).
- Organize by site as a top-level grouping option.

### 14. Watch Folder / Auto-Import

**Area:** Library  
**Suggestion:** Monitor a configurable directory for new video/image files. When new files appear, automatically import them into the library:

- Probe metadata (duration, resolution, thumbnail).
- Attempt auto-tagging from filename.
- Show import progress in a dedicated panel.
- Configurable file extensions to watch.

### 15. Scene Notes / Annotations

**Area:** Library  
**Suggestion:** Add a free-text `notes` field to scenes. Users can annotate scenes with personal notes, ratings, or timestamps of interesting moments. Notes are searchable via FTS5.

### 16. Per-Scene Custom Thumbnail

**Area:** Library  
**Currently:** Thumbnails are auto-generated (remote URL → probe frame).  
**Suggestion:** Allow users to:

- Upload a custom thumbnail image for a scene.
- Scrub through the video to select a specific frame.
- Set thumbnail from a URL.

### 17. Theme Customization

**Area:** Frontend / Settings  
**Currently:** Dark-first with oklch tokens.  
**Suggestion:**

- Light/dark toggle in Settings (not just system preference).
- Custom accent color picker.
- Font size / density settings (compact, comfortable, spacious).
- Custom CSS injection for power users.

### 18. Plugin Marketplace / Registry

**Area:** Plugins  
**Currently:** Manual `git clone` into `plugins/`.  
**Suggestion:**

- In-app plugin browser that lists available community plugins from a JSON registry.
- One-click install (auto-clone + `plugins:generate`).
- Plugin version checking and update notifications.
- Plugin enable/disable toggle without deleting.

### 19. Scheduled Downloads

**Area:** Downloads  
**Suggestion:** Queue downloads to start at a specific time or on a schedule. Useful for:

- Off-peak bandwidth usage.
- Recurring downloads (e.g., daily channel check).
- Pause during certain hours (e.g., no downloads during work hours).

### 20. Mobile: Offline Queue Sync

**Area:** Mobile / LAN  
**Currently:** Mobile standalone mode is limited to direct URL resolve.  
**Suggestion:** When the mobile client reconnects to LAN after being offline, sync any queued download requests. Allow users to queue URLs while offline (standalone mode stores them locally), then batch-send to the desktop when reconnected.

---

## Infrastructure / Technical Improvements

### 21. Database Migration Tooling

**Area:** Rust / DB  
**Suggestion:** Add a `db migrate` CLI command (via Tauri CLI or standalone) to:

- Run pending migrations without starting the full app.
- Roll back migrations.
- Validate migration integrity.

### 22. Automated Backup

**Area:** Library  
**Suggestion:** Scheduled SQLite backup (copy the `.db` file to a backup directory). Configurable backup interval and retention. One-click restore from backup in Settings.

### 23. Performance Monitoring Dashboard

**Area:** Settings  
**Suggestion:** Show in Settings:

- Library stats (scene count, total size, free space, duplicate count).
- Download queue stats (active, pending, completed, failed).
- LAN connection stats (active clients, bandwidth usage).
- Database size and query performance metrics.

### 24. Content Rating / Parental Controls

**Area:** Library / Settings  
**Suggestion:** Add a content rating system (e.g., SFW / NSFW / Explicit). Allow users to:

- Tag scenes with a rating.
- Filter by rating in browse and library views.
- Set a default rating filter per session.
- Lock certain ratings with a PIN (parental controls).

### 25. REST API Pagination & Filtering

**Area:** LAN Server  
**Suggestion:** Add query parameters to LAN API endpoints:

- `?page=1&per_page=50` for paginated responses.
- `?sort=newest&order=desc` for sorting.
- `?performer=X&tag=Y` for filtering.
- `?search=query` for search.
- This would make the LAN API more useful for third-party integrations.

---

## Quick Wins (< 1 day each)

| # | Feature | Description |
|---|---------|-------------|
| Q1 | **Scene count badge** | Show total scene count on the Library nav item. |
| Q2 | **Last downloaded sort** | Add "Recently Downloaded" sort option to scenes. |
| Q3 | **File size display** | Show file size on SceneCard in grid/list view. |
| Q4 | **Performer image upload** | Allow setting performer profile images manually. |
| Q5 | **Dark/light theme toggle** | Simple toggle button in the header bar. |
| Q6 | **Keyboard shortcut hints** | Show shortcut keys in tooltips and menus. |
| Q7 | **Export performer list** | Simple CSV/JSON export of all performers. |
| Q8 | **Scene duration filter** | Add min/max duration inputs to the scenes filter bar. |
| Q9 | **Empty state illustrations** | Add friendly illustrations to empty states. |
| Q10 | **Changelog in-app** | Show recent changes on first launch after update. |

---

*Last updated: 2026-08-11*
