export type BrowseKind = "tag" | "model" | "channel" | "search" | "video";

export interface MediaItem {
  id: string;
  title: string;
  url: string;
  thumbnail?: string;
  duration?: number;
  site_id: string;
  performers: string[];
  tags: string[];
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

export type DownloadStatus = "pending" | "active" | "completed" | "failed" | "cancelled";

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

export interface AppSettings {
  engine_mode: EngineMode;
  library_path: string;
  naming_template: string;
  lan_enabled: boolean;
  lan_port: number;
  lan_token?: string;
  remote_host?: string;
  remote_token?: string;
}

export interface DuplicateGroup {
  match_type: string;
  hash: string;
  scenes: Scene[];
}

export interface CookieSiteInfo {
  site_id: string;
  updated_at: string;
}

export interface HealthResponse {
  status: string;
  version: string;
}
