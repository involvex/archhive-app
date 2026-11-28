# ArcHive Roadmap

> **Status:** Curated, verified backlog — every item below was checked against the code on **2026-11-28** (repo at `v0.9.4`, branch `main`).
> **Rule:** Code is the source of truth for status. `suggestions.md` is a historical idea list and is intentionally left unchanged; several of its status labels are stale (see [Stale claims](#stale-claims-in-suggestionsmd)).
> **Scope:** "Realistically shippable" means no new dependency, no native/Kotlin work, and no DB migration unless the item says so explicitly in its blocker column.

---

## Legend

| Symbol        | Meaning                                                                 |
| ------------- | ----------------------------------------------------------------------- |
| **Wave 0**    | Pure frontend or one-line fixes — no migration, native code, or new dep |
| **Wave 1**    | Small Rust + TS change, no new dep and no migration                     |
| **Wave 2**    | Needs a migration, native code, or a larger design — not a quick win    |
| **Evidence**  | Existing file/symbol that makes the item cheap to ship                  |
| **Done when** | Concrete acceptance criterion that can be verified                      |

Effort is a rough guide: **S** (< 1 day), **M** (1–3 days), **L** (> 3 days / multi-file subsystem).

---

## Wave 0 — Pure frontend / one-line fixes

No DB migration, no native code, no new dependency. Each of these can ship independently.

| ID  | Item                                                   | Source      | Effort | Evidence (why doable)                                                                                             | Done when                                                                                    |
| --- | ------------------------------------------------------ | ----------- | ------ | ----------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| R01 | Fix backup manifest schema id (021 → 022)              | stale bug   | S      | `src-tauri/src/library/backup.rs:25` has `CURRENT_MIGRATION_ID = "021"`; latest is `MIGRATION_022`                | Export backup → `manifest.json` reports `022`; round-trip test still passes                  |
| R02 | Naming-template presets + token chip picker            | #9          | S      | `src/lib/namingTemplate.ts:1–125` already validates/renders tokens; Settings UI at `settings/index.tsx:1790–1816` | Save/load/rename presets; click a chip inserts its token into the template                   |
| R03 | _(shipped — see appendix)_                             |             |        |                                                                                                                   |                                                                                              |
| R04 | _(shipped — see appendix)_                             |             |        |                                                                                                                   |                                                                                              |
| R05 | _(shipped — see appendix)_                             |             |        |                                                                                                                   |                                                                                              |
| R06 | Saved-search JSON export / import                      | #28 partial | S      | `listSavedSearches`/`saveSearch` exist; Diagnostics already hosts settings/cookies backup UI                      | Export downloads JSON; import restores searches with validation                              |
| R07 | Post-wizard Home checklist                             | #40         | S      | `FirstRunWizardGate` + `/` stats grid + `useSettingsStore` already exist                                          | Dismissible checklist card with deep links (library path, engine, cookies, LAN)              |
| R08 | Auto-suggest landscape on rotate + 44px player targets | #44         | S      | `useIsLandscape()` at `src/lib/hooks/useMediaQuery.ts:42`; player Prev/Next are 40px, action bar 44px             | Rotating to landscape suggests fullscreen; all player touch targets ≥ 44px                   |
| R09 | _(moved to Wave 1 — see note below)_                   |             |        |                                                                                                                   |                                                                                              |
| R10 | _(already shipped — moved to appendix)_                |             |        |                                                                                                                   |                                                                                              |
| R11 | App Lock / Private Mode — PIN gate + blur              | #53 Phase 1 | M      | `vault_pin_hash`/`verify_vault_pin` + `authenticateForVault()` exist; gate pattern in `FirstRunWizardGate`        | PIN on cold start/background return (configurable timeout); Private mode blurs thumbs        |
| R12 | Offline outbox for Remote LAN                          | #20/#36     | M      | `shouldUseRemoteApi()` switch, browse-cache precedent, `useLanConnection` reconnect trigger, localStorage helpers | Queue while offline; badge shows count; flush on reconnect with per-item receipts/ conflicts |
| R13 | Player "select frame" scrubber UI                      | #16/#31     | M      | `setSceneThumbFromFrame` + "Use this frame" button already work                                                   | Scrubber picks an arbitrary frame and writes `{stem}.jpg`                                    |

---

## Wave 1 — Small Rust + TS, no new dependency or migration

| ID  | Item                                          | Source     | Effort | Evidence (why doable)                                                                                      | Done when                                                                                                        |
| --- | --------------------------------------------- | ---------- | ------ | ---------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| R14 | Custom thumbnail from URL / image upload      | #16        | S      | `download_remote_thumbnail` + `is_valid_image_bytes` + sidecar/cleaner already exist                       | `set_scene_thumb_from_url` command + LAN route + client wrapper; upload writes `{stem}.jpg`                      |
| R15 | Storyboard thumbnails + hover scrub           | #31        | M      | `FfmpegProcessor` + `spawn_ffmpeg` progress exist; no `tile` method yet                                    | Contact-sheet sidecar on import/probe; `SceneCard` hover shows scrub preview                                     |
| R16 | GET `/api/scenes` query-param aliases         | #25        | S      | `SceneFilter` + POST `/api/library/filter` + parameterized capped LIMIT/OFFSET already exist               | GET accepts `page`, `per_page`, `min_duration`, `tag`, `performer`, `min_rating`, `hide_watched`                 |
| R17 | Scheduled backup with retention               | #22/#32    | M      | `export_library_backup`/dry-run/restore complete; only settings + tokio task + Settings card missing       | Configurable interval/retention; old backups pruned; no new dependency                                           |
| R18 | Performance dashboard subset                  | #23        | S      | `get_library_stats`, `/api/duplicates`, `/api/downloads`, `/api/library/*` already expose the data         | Settings → Diagnostics shows library, duplicate, download-queue and storage cards                                |
| R19 | Post-download auto-organize (move/copy)       | #13        | M      | `update_scene(rename_file)` + naming tokens + import pipeline exist                                        | Shared `render_naming_template(scene)`; optional move/copy into `{performer}/{title}.{ext}` with sidecars        |
| R20 | Fuzzy alias suggestions + studio from URL     | #1/#7      | M      | Performer aliases, `source_url`, `SiteRegistry.base_url()` all stored/exposed                              | Suggestions match normalized aliases; completion pipeline fills studio from domain with aggregate-site allowlist |
| R21 | Studio editing + browse-side studio browser   | #7         | S      | `upsert_studio`/`list_studios` + site catalog exist; `UpdateSceneRequest` has no studio field              | Studio editable in Scene Edit; Browse has a studio browser                                                       |
| R22 | Per-site naming template overrides            | #9         | M      | `AppSettings` is serde-defaulted JSON; `manager.rs:288` already has `job.adapter`                          | Override per site in Settings; download uses that site's template                                                |
| R23 | Configurable shortcuts + fuzzy palette jump   | #10        | M      | `shortcuts/registry.ts` supports enable/disable + re-register; client-side list filtering                  | Shortcut map in Settings; palette jumps to scenes/performers/tags by name                                        |
| R24 | Alias editor + tag reparent + bulk retag      | #29        | S      | `setTagParent` + `batch_update_scenes` already exist                                                       | Performer aliases editable; tag tree drag-to-reparent; bulk retag from selection                                 |
| R25 | Optimize / transcode action                   | #35        | M      | ffmpeg sidecar bundled; DB updaters (`update_scene_file_size`/`duration`) exist; zero transcode code today | `FfmpegProcessor::transcode` with progress; per-scene + bulk action; keep original option                        |
| R26 | Bulk rename/move + orphan cleaner in Files UI | #39        | M      | `naming.rs` + safe rename + orphan commands exist; `/files` is read-only today                             | Apply naming template to selection; orphan sidecars selectable/deletable with size totals                        |
| R27 | Manual "Record live" (capped + cancellable)   | #30 manual | M      | `DownloadTool::FfmpegHls`, `download_hls`, `resolve_livestream`, import pipeline all exist                 | `record_livestream` builds HLS plan; `-t` cap; cancel kills ffmpeg; partial file imports                         |
| R28 | LAN scoped read-only tokens                   | #37 core   | M      | `ApiState` + loopback read-only allowlist is a template; `AppSettings` serde defaults avoid migration      | `read`/`admin` tokens with labels + revoke list in Settings → LAN                                                |
| R29 | Migration runner + integrity check            | #21        | M      | `MIGRATION_001`–`022` + `column_exists` guards exist; no CLI/bin today                                     | Extract `run_migrations(conn)`; `src/bin` runner with `integrity_check`; pending ledger                          |
| R30 | Embedded per-site cookie import in wizard     | #40        | M      | Inline import UI at `settings/index.tsx:2822–2930`; `api.listSites`/`saveSiteCookies` exist                | Wizard step imports cookies without linking out                                                                  |
| R09 | Data-saver-aware thumbnails                   | #54 rem.   | M      | `data_saver` shipped, but `thumb_quality` is a **no-op setting** — never read by `library/thumbnail.rs`    | Backend generates a low-quality sidecar using `thumb_quality`; `sceneThumbUrl()` requests it when data-saver on  |

> **R09 note:** the pre-verification assumption that this was frontend-only was wrong. `src-tauri/src/library/thumbnail.rs` writes the downloaded bytes verbatim and `extract_thumbnail` has no quality parameter, so a real low-quality rendition needs backend work. The roadmap has been corrected accordingly.

---

## Wave 2 — Migration, native code, or larger design

These are deliberately **not** promised as quick wins. Each needs the named blocker resolved first.

| ID  | Item                                     | Source     | Effort | Blocker                                                                     | Done when                                                         |
| --- | ---------------------------------------- | ---------- | ------ | --------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| R31 | SFW / NSFW / Explicit content rating     | #24        | L      | `MIGRATION_023` + model mirror (`models.rs`/`types.ts`) + filter + PIN gate | Content rating set/filtered; per-session default; PIN enforcement |
| R32 | "Start after HH:MM" on pending jobs      | #19        | M      | `DownloadJob` field + migration + scheduler tick (poller pattern exists)    | Scheduled job stays deferred until its time, then starts          |
| R33 | Subtitle fetch + `<track>` display       | #38        | M      | `DownloadPlan` fields, yt-dlp `--write-subs`, VTT MIME + scene route        | `.vtt` sidecar fetched; player language picker; LAN serves it     |
| R34 | Save to Movies / Gallery                 | #51        | M      | New Kotlin MediaStore command in `android-overlays/YtDlpPlugin.kt`          | On-device scene exported to public Movies with share              |
| R35 | `ACTION_VIEW` share intent               | #47        | S      | Extend `scripts/patch-android-share.ps1` manifest filters                   | Shared media URL/file opens ArcHive                               |
| R36 | Pause/cancel actions on FG notification  | #55 polish | M      | Native `android-overlays/DownloadForegroundService.kt`                      | Ongoing notification has pause/cancel; grouped summary            |
| R37 | LAN rate limiting + audit log            | #37 tail   | M      | New dep or hand-rolled limiter; `LogBuffer` sink exists                     | Queue endpoints rate-limited; remote actions auditable            |
| R38 | On-the-fly low-bandwidth LAN transcode   | #35 tail   | L      | New streaming pipeline + cache + cancellation                               | `?quality=low` serves a cached rendition                          |
| R39 | Trash-with-restore + undo for bulk ops   | #39 tail   | L      | New trash location/manifest + retention lifecycle + LAN parity              | 30-day restore and undo toast for last bulk op                    |
| R40 | Scheduled / auto-record favorited models | #30 tail   | L      | New availability watcher + schedule subsystem                               | Favorited model recorded automatically when online in window      |

---

## Already shipped — not roadmap

Recorded here so planning stops re-litigating them. Evidence lives in code and the in-app changelog (`src/lib/changelog.ts`).

- Collections + Watchlists (#2, #6, #27, #42), Watch history / resume (#4, #26)
- Download queue reorder, concurrent cap, retry/backoff, global rate limit (#3)
- Improvements to duplicates UI (#5), notes + timestamp seek (#15)
- Keyboard shortcuts / command palette (#10 base), notifications + quiet hours (#11, #43)
- Advanced scene filters + FTS (#12), watch-folder auto-import (#14)
- Tag hierarchy + performer merge (#29 base), live browse + HLS playback (#30 base)
- Log viewer / diagnostics (#33), binary update manager (#34)
- Full DB backup/restore (#32 base), first-run wizard (#40 base), smart collections (#41 base)
- AMOLED theme + Data-Saver core (#54), Android FG service + notification channels (#55 core)
- Share-sheet `SEND` intent + one-tap queue from prefill (`BulkImportPanel.tsx:31–39`, `151–159`) (#47 core/delta), "Open with system player" (#51 core)
- PiP, gestures, wake-lock, fullscreen (#44 core), duplicate count badge (#23 partial)
- Pagination base + thumb quality (#25 base, #49 Phase 1)
- Wi-Fi-only / battery-aware downloads (#45), LAN history + QR pairing (#46, Q35/Q36)
- **Shipped from this roadmap (v0.9.4 dev):** R03 bulk suggest-and-apply tags (`SceneBulkEditBar.tsx` + test), R04 re-queue failed URLs (`downloads/index.tsx`), R05 copy/Open HLS stream URL (`live/$site/$slug.tsx`)

---

## Stale claims in `suggestions.md`

Documented, not edited. The roadmap above supersedes these.

| `suggestions.md` location | Claim                                 | Reality                                                                    |
| ------------------------- | ------------------------------------- | -------------------------------------------------------------------------- |
| #21                       | "MIGRATION_001–007"                   | Now `MIGRATION_001`–`022`                                                  |
| #23                       | "duplicate count — surface the count" | Sidebar badge already shipped (Q14)                                        |
| #25                       | "some attribute filters" on GET       | `cup_size`/`hair_color` belong to `list_performers`, not `GET /api/scenes` |
| #41 body                  | rating "never written or displayed"   | Implemented via Q27                                                        |
| #53                       | "needs a native biometric plugin"     | `@tauri-apps/plugin-biometric` already wired for the vault                 |
| #54                       | AMOLED + Data-Saver "Not started"     | Both shipped (`src/lib/amoled.ts`, `AppSettings.data_saver`)               |
| #55                       | FG notifications "Partial"            | Foreground service + channels shipped; only pause/cancel actions remain    |
| #44/#47/#51               | "Not started" / "Partial"             | PiP/gestures, `SEND` intent, and open-with are shipped                     |

---

## Verification

Before marking any item complete, run the full local gate:

```powershell
bun run typecheck
bun run lint
bun run test          # Vitest — NOT `bun test` (no jsdom / vi.* helpers)
bun run format:check
cd src-tauri && cargo test   # requires a built dist/ (already present)
```

For Wave 2 items, also run:

```powershell
bun run android:validate
bun run build:apk:fast
```

Physical-device testing is done by the user; do not install APKs or launch activities on their device.

---

## Maintenance

- Move an item to **Already shipped** only after its **Done when** criterion passes.
- When a new dependency or migration becomes unavoidable during implementation, move the item to **Wave 2** and record the reason.
- Keep `models.rs` and `src/lib/types.ts` in sync for every new field; add settings with `#[serde(default)]` to avoid migrations where possible.
- Never edit `src/routeTree.gen.ts`.

_Last verified: 2026-11-28 (v0.9.4)._
