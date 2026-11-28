export interface ChangelogEntry {
  version: string;
  date: string;
  changes: { type: "added" | "fixed" | "changed"; text: string }[];
}

export const CHANGELOG: ChangelogEntry[] = [
  {
    version: "0.9.4",
    date: "2026-11-27",
    changes: [
      {
        type: "added",
        text: "Quiet hours in Settings → Downloads → Notifications: mute OS and in-app alerts on a schedule (overnight windows supported)",
      },
      {
        type: "added",
        text: "Optional pause of watchlist auto-queue during quiet hours (manual Check now still runs)",
      },
      {
        type: "added",
        text: "Notification toggles for watch-folder auto-import and library scan finished",
      },
    ],
  },
  {
    version: "0.9.3",
    date: "2026-11-26",
    changes: [
      {
        type: "added",
        text: "Watch folder auto-import (desktop): opt-in Settings → Library toggle watches the library (or a subfolder) and imports settled video files with filename tags + thumbnails",
      },
    ],
  },
  {
    version: "0.9.2",
    date: "2026-11-26",
    changes: [
      {
        type: "added",
        text: "Drag (or arrow) reorder for pending and waiting-for-Wi-Fi downloads; scheduler starts the front of the queue",
      },
      {
        type: "added",
        text: "Global yt-dlp download speed limit in Settings → Downloads (e.g. 500K / 2M)",
      },
    ],
  },
  {
    version: "0.9.0",
    date: "2026-11-22",
    changes: [
      {
        type: "added",
        text: "Click-to-seek [mm:ss] / [h:mm:ss] timestamps in scene notes from the library player",
      },
      {
        type: "added",
        text: "Naming template live preview with unknown-token and illegal-path warnings",
      },
      {
        type: "added",
        text: "Use this frame — set a scene thumbnail from the current player position",
      },
      {
        type: "added",
        text: "Side-by-side muted preview on Duplicate Keep/Remove cards",
      },
      {
        type: "added",
        text: "Watchlist poller notifications plus Settings toggles for download and match alerts",
      },
      {
        type: "added",
        text: "Watchlist auto-queue poller on Android (skips when Wi-Fi-only and offline Wi-Fi)",
      },
      {
        type: "fixed",
        text: "RedGifs no longer queues gallery-dl on Android when a direct URL is unavailable",
      },
    ],
  },
  {
    version: "0.8.0",
    date: "2026-11-22",
    changes: [
      {
        type: "added",
        text: "Full library database backup/restore (.archhive-backup.zip) with path remap and safety copy",
      },
      {
        type: "added",
        text: "JSON export of scenes, performers, and tags (Settings → Diagnostics)",
      },
      {
        type: "added",
        text: "Duplicate Keep best (resolution → size) per group and apply-all, with size/resolution/duration badges",
      },
      {
        type: "added",
        text: "Merge performers from the detail page; tag parent editor with indented tag tree",
      },
      {
        type: "added",
        text: "Configurable concurrent downloads (1–4) in Settings → Downloads",
      },
    ],
  },
  {
    version: "0.7.0",
    date: "2026-11-15",
    changes: [
      {
        type: "added",
        text: "Batch remove performers/tags from the library multi-select bar",
      },
      {
        type: "added",
        text: "Collection cover mosaics, drag-to-reorder, and bulk Add to collection",
      },
      {
        type: "added",
        text: "Studios browser (virtual studios from studio name / channel)",
      },
      {
        type: "added",
        text: "Auto-tag from yt-dlp metadata (toggle in Settings) plus suggested tags in scene edit",
      },
      {
        type: "added",
        text: "Vault unlock gate: one-shot backend token + optional/desktop PIN",
      },
      {
        type: "changed",
        text: "Vaulted media reads refuse access while locked (path + is_vaulted flag)",
      },
    ],
  },
  {
    version: "0.6.6",
    date: "2026-11-01",
    changes: [
      {
        type: "added",
        text: "Media vault with biometric lock, gallery hiding, and Android MediaStore cleanup",
      },
      {
        type: "added",
        text: "CamGirlFinder Explore (search / face / random / profile)",
      },
      {
        type: "added",
        text: "In-app cam viewer improvements and Stripchat availability fixes",
      },
      {
        type: "fixed",
        text: "Android vault original-file deletion and ProGuard/yt-dlp packaging overlays",
      },
    ],
  },
  {
    version: "0.6.5",
    date: "2026-10-20",
    changes: [
      {
        type: "added",
        text: "Native Stripchat HLS resolver and CamsFinder Live tab source",
      },
      {
        type: "fixed",
        text: "Fullscreen player invoke import on Android",
      },
    ],
  },
  {
    version: "0.6.4",
    date: "2026-10-12",
    changes: [
      {
        type: "added",
        text: "In-app cookie capture on Android, ThotHub session fixes, optional Termux handoff",
      },
      {
        type: "added",
        text: "Browse filters, tag indexes, and category browsers across major tube sites",
      },
      {
        type: "changed",
        text: "Standalone engine mode retired; phones default to on-device local engine",
      },
      {
        type: "fixed",
        text: "Mobile library playback without LAN hosting, ThotHub/KVS watch paths",
      },
    ],
  },
  {
    version: "0.6.3",
    date: "2026-10-01",
    changes: [
      {
        type: "fixed",
        text: "Android release packaging and plugin Args retention for Jackson",
      },
    ],
  },
  {
    version: "0.6.2",
    date: "2026-09-28",
    changes: [
      {
        type: "fixed",
        text: "Legacy native-lib packaging so Chaquopy finds libpython on Android",
      },
    ],
  },
  {
    version: "0.6.1",
    date: "2026-09-25",
    changes: [
      {
        type: "changed",
        text: "Tauri CSP enabled for desktop/Android webviews",
      },
      {
        type: "fixed",
        text: "Android release boot crash from R8 stripping commons-compress",
      },
    ],
  },
  {
    version: "0.6.0",
    date: "2026-09-20",
    changes: [
      {
        type: "added",
        text: "Recommendations Phase 1–2: library picks rail, ranked live/feed slices",
      },
      {
        type: "added",
        text: "Auto-sign release APK helper for local builds",
      },
    ],
  },
  {
    version: "0.5.3",
    date: "2026-10-05",
    changes: [
      {
        type: "added",
        text: "Cup & Color Finder in Browse with per-site external search links, recent searches, and library matches",
      },
      {
        type: "added",
        text: "Performer cup size / hair color attributes with library filtering, detail editor, and LAN API",
      },
      {
        type: "fixed",
        text: "External Open links now open in the system browser via the opener plugin on desktop and Android",
      },
    ],
  },
  {
    version: "0.5.2",
    date: "2026-09-23",
    changes: [
      {
        type: "fixed",
        text: "Settings no longer walks the library on every open — storage/orphan scans run only on the Library tab",
      },
      {
        type: "changed",
        text: "Orphan and thumb-cache totals share one filesystem walk via library_storage_stats",
      },
    ],
  },
  {
    version: "0.5.1",
    date: "2026-09-15",
    changes: [
      {
        type: "fixed",
        text: "In-app video player loads media again (library, live, and browse) with working volume controls",
      },
      {
        type: "fixed",
        text: "ThotHub Watch works on Android via on-device KVS stream extraction (no Remote LAN required)",
      },
      {
        type: "changed",
        text: "Clearer stream-resolve errors when a site cannot play on-device",
      },
    ],
  },
  {
    version: "0.5.0",
    date: "2026-09-11",
    changes: [
      {
        type: "added",
        text: "Dark mode schedule (sunrise-to-sunset or custom hours) in Settings → Appearance",
      },
      { type: "added", text: "CSV export for performers in Library → Performers" },
      { type: "added", text: "Clear completed downloads button in the Downloads page" },
      { type: "added", text: "1-5 star rating system for scenes, editable from Scene Details" },
      { type: "added", text: "Watched/unwatched count badge in the Scenes filter bar" },
      {
        type: "added",
        text: "Spacebar play/pause and arrow-seek keyboard controls in the video player",
      },
      { type: "added", text: "Auto-advance to next scene on playback end (Settings → Player)" },
      { type: "added", text: "Mark watched / unwatched toggle in the video player dialog" },
      { type: "added", text: "Source-site badge on scene cards" },
      { type: "added", text: "'W' keyboard shortcut to toggle watched status on selected scenes" },
      { type: "added", text: "Dark mode schedule theme option" },
      { type: "added", text: "Searchable notes field for scenes (FTS5 indexed)" },
      { type: "added", text: "Changelog dialog (shows on first launch after an update)" },
      { type: "fixed", text: "Video player no longer steals focus from the dialog container" },
      {
        type: "changed",
        text: "Refactored player dialog to lift videoRef for reliable playback control",
      },
    ],
  },
  {
    version: "0.4.0",
    date: "2026-06-01",
    changes: [
      {
        type: "added",
        text: "Initial release: browse, download, and organize media from 15+ sites",
      },
      { type: "added", text: "Local library with SQLite + FTS5 full-text search" },
      { type: "added", text: "Optional LAN REST API for mobile clients" },
    ],
  },
];

export const LAST_VERSION_KEY = "archhive:changelog:lastVersion";

export function getLatestVersion(): string {
  return CHANGELOG[0].version;
}

export function isNewVersion(appVersion: string): boolean {
  if (appVersion === "dev" || appVersion === "…") return false;
  const seen = localStorage.getItem(LAST_VERSION_KEY);
  return seen !== appVersion;
}

export function markVersionAsSeen(appVersion: string): void {
  localStorage.setItem(LAST_VERSION_KEY, appVersion);
}

export function getChangelogEntriesForVersion(version: string): ChangelogEntry[] {
  const entry = CHANGELOG.find((c) => c.version === version);
  return entry ? [entry] : [];
}
