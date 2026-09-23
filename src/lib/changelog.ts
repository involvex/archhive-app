export interface ChangelogEntry {
  version: string;
  date: string;
  changes: { type: "added" | "fixed" | "changed"; text: string }[];
}

export const CHANGELOG: ChangelogEntry[] = [
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
