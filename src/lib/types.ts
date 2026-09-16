export type BrowseKind =
  "tag" | "model" | "channel" | "search" | "video" | "category" | "livestream";

export type BrowseOrientation = "straight" | "gay" | "lesbian" | "transgender";

export interface MediaItem {
  id: string;
  title: string;
  url: string;
  thumbnail?: string;
  duration?: number;
  site_id: string;
  performers: string[];
  tags: string[];
  description?: string;
  channel?: string;
  is_live?: boolean;
  viewers?: number;
  age?: number;
  gender?: string;
  stream_url?: string;
  embed_url?: string;
}

export interface BrowsePage {
  items: MediaItem[];
  page: number;
  has_more: boolean;
  total?: number;
}

export interface SiteInfo {
  id: string;
  display_name: string;
  base_url: string;
  supported_kinds: BrowseKind[];
  requires_cookies: boolean;
}

export type DownloadStatus =
  "pending" | "active" | "paused" | "waiting_for_wifi" | "completed" | "failed" | "cancelled";

export interface BulkImportResult {
  queued: number;
  expanded: number;
  skipped: number;
}

export interface DownloadJob {
  id: string;
  url: string;
  adapter: string;
  status: DownloadStatus;
  progress: number;
  output_path?: string;
  error?: string;
  title?: string;
  created_at: string;
  retry_count?: number;
  last_retry_at?: string;
}

export interface Scene {
  id: string;
  title: string;
  path?: string;
  duration?: number;
  thumb?: string;
  source_url?: string;
  studio_id?: string;
  studio_name?: string;
  date?: string;
  rating?: number;
  performers: string[];
  tags: string[];
  channel?: string;
  phash?: string;
  oshash?: string;
  file_size?: number;
  notes?: string;
  width?: number;
  height?: number;
}

export interface UpdateSceneRequest {
  title?: string;
  performers?: string[];
  tags?: string[];
  rename_file?: boolean;
  notes?: string;
  rating?: number;
}

export interface BatchUpdateScenesRequest {
  scene_ids: string[];
  performers_add?: string[];
  tags_add?: string[];
}

export interface BatchUpdateScenesResult {
  updated: number;
}

export interface PornhubCategoryEntry {
  name: string;
  slug: string;
  orientation: BrowseOrientation;
  category_id?: number;
  video_count?: number;
}

export interface Performer {
  id: string;
  name: string;
  aliases: string[];
  image?: string;
  favorite: boolean;
  scene_count: number;
}

export interface Tag {
  id: string;
  name: string;
  parent_id?: string;
  scene_count: number;
}

export interface Studio {
  id: string;
  name: string;
  url?: string;
}

export type EngineMode = "local" | "remote_lan" | "standalone";

export type SceneSort = "newest" | "name" | "downloaded";

export type DownloadQuality = "best" | "1080" | "720" | "480";
export type ThumbnailQuality = "original" | "low" | "medium" | "high";

export type AppTheme = "dark" | "light" | "system" | "scheduled";

export interface AppSettings {
  engine_mode: EngineMode;
  library_path: string;
  naming_template: string;
  lan_enabled: boolean;
  lan_port: number;
  lan_token?: string;
  remote_host?: string;
  remote_token?: string;
  phash_threshold?: number;
  close_to_tray?: boolean;
  minimize_to_tray?: boolean;
  tray_hotkey?: string;
  download_quality?: DownloadQuality;
  prefer_mp4?: boolean;
  lan_auth_enabled?: boolean;
  auto_tag_rules?: string[];
  trending_sites?: string[];
  download_max_retries?: number;
  download_retry_delay_seconds?: number;
  theme?: AppTheme;
  watched_threshold?: number;
  watch_poll_interval_mins?: number;
  /** Auto-advance to next scene in playlist when playback ends (#24). */
  auto_advance_next?: boolean;
  /** Hold a screen wake lock while the player is open (Q33). */
  keep_screen_on?: boolean;
  /** Dark schedule start time HH:MM (e.g. "19:00"). (#29) */
  theme_schedule_from?: string;
  /** Light schedule start time HH:MM (e.g. "07:00"). (#29) */
  theme_schedule_to?: string;
  /** Download only on Wi-Fi (default ON on mobile). (#45) */
  download_on_wifi_only?: boolean;
  /** Pause downloads when battery saver is active (default ON on mobile). (#45) */
  pause_on_battery_saver?: boolean;
  /** Thumbnail quality for library grid (default original). (#49) */
  thumb_quality?: ThumbnailQuality;
}

export interface MergeDuplicatesResult {
  removed: number;
}

export interface DuplicateGroup {
  match_type: string;
  hash: string;
  scenes: Scene[];
  max_distance?: number;
}

export interface CookieSiteInfo {
  site_id: string;
  updated_at: string;
}

export interface HealthResponse {
  status: string;
  version: string;
  auth_required?: boolean;
  lan?: boolean;
  library_path?: string;
  lan_url?: string;
}

export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size?: number;
  mime?: string;
}

export interface FilesListResponse {
  path: string;
  entries: FileEntry[];
}

export interface LanHost {
  name: string;
  url: string;
  ip: string;
  port: number;
}

export interface SceneFilter {
  missing_thumb?: boolean;
  missing_duration?: boolean;
  min_duration?: number;
  max_duration?: number;
  hash_named?: boolean;
  hide_watched?: boolean;
  performer_names?: string[];
  tag_names?: string[];
  min_rating?: number;
  min_file_size?: number;
  collection_id?: string;
}

export type CollectionType = "collection" | "watchlist" | "smart";

export interface Collection {
  id: string;
  name: string;
  type: CollectionType;
  description?: string;
  filter_json?: string;
  filter?: SceneFilter;
  scene_count: number;
  created_at: string;
  updated_at: string;
}

export interface CreateCollectionRequest {
  name: string;
  type?: CollectionType;
  description?: string;
  filter?: SceneFilter;
}

export interface UpdateCollectionRequest {
  name?: string;
  description?: string;
  filter?: SceneFilter;
}

export interface ExportCollectionResult {
  content: string;
  filename: string;
}

export interface FfmpegStatus {
  ffmpeg_available: boolean;
  ffprobe_available: boolean;
}

export interface BinaryVersions {
  ffmpeg_version?: string;
  ffprobe_version?: string;
  ytdlp_version?: string;
  gallery_dl_version?: string;
}

export interface DirEntry {
  name: string;
  path: string;
}

export interface DirBrowseResponse {
  current: string;
  parent: string | null;
  dirs: DirEntry[];
  can_write: boolean;
  roots: DirEntry[];
}

export interface ClearThumbsResult {
  cleared: number;
}

/** Q43: thumbnail sidecar cache totals (mirrors Rust ThumbCacheStats). */
export interface ThumbCacheStats {
  file_count: number;
  total_bytes: number;
}

export interface SidecarProbe {
  name: string;
  bundled: boolean;
  detail: string;
}

export interface WatchProgress {
  scene_id: string;
  position_secs: number;
  duration_secs: number;
  watched: boolean;
  updated_at: string;
}

export interface MarkWatchedResult {
  updated: number;
}

export interface SavedSearch {
  id: string;
  name: string;
  site_id: string;
  kind: BrowseKind;
  slug: string;
  orientation?: BrowseOrientation;
  last_checked_at?: string;
  new_count: number;
  auto_queue: boolean;
  created_at: string;
}

export interface WatchlistPollResult {
  checked: number;
  queued: number;
  errors: number;
}

export interface WatchPollRun {
  finished_at: string;
  checked: number;
  queued: number;
  errors: number;
}

export interface WatchlistStatus {
  last_run?: WatchPollRun;
  auto_queue_count: number;
  due_count: number;
}

export interface CheckSavedSearchResult {
  search_id: string;
  new_items: MediaItem[];
  new_count: number;
  total: number;
  checked_at: string;
}

export interface ThumbGenResult {
  generated: number;
  errors: number;
}

export interface DurationProbeResult {
  probed: number;
  errors: number;
}

export interface OrphanSidecar {
  path: string;
  size: number;
}

export interface LibraryStats {
  scene_count: number;
  performer_count: number;
  tag_count: number;
  total_size_bytes: number;
  free_space_bytes: number;
}

export interface LogEntry {
  timestamp: number;
  level: string;
  target: string;
  message: string;
}

export interface DiagnosticsData {
  app_version: string;
  binary_versions: BinaryVersions;
  ffmpeg_status: FfmpegStatus | null;
  library_stats: LibraryStats | null;
  lan_enabled: boolean;
  lan_port: number;
  engine_mode: string;
  library_path_set: boolean;
  cookies_configured: boolean;
  recent_logs: LogEntry[];
}

export interface CookieBackupEntry {
  site_id: string;
  netscape: string;
}

export interface SettingsBackup {
  schema_version: number;
  app_version: string;
  exported_at: string;
  settings: AppSettings;
  cookies: CookieBackupEntry[];
}

export interface SettingsBackupImportOptions {
  include_remote_credentials?: boolean;
}

export interface SettingsBackupImportResult {
  settings_applied: boolean;
  cookies_imported: number;
  library_path_skipped: boolean;
}
