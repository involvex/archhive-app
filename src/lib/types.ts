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

export type DownloadStatus = "pending" | "active" | "paused" | "completed" | "failed" | "cancelled";

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
}

export interface UpdateSceneRequest {
  title?: string;
  performers?: string[];
  tags?: string[];
  rename_file?: boolean;
  notes?: string;
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

export type AppTheme = "dark" | "light" | "system";

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
  performer_names?: string[];
  tag_names?: string[];
}

export interface FfmpegStatus {
  ffmpeg_available: boolean;
  ffprobe_available: boolean;
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
