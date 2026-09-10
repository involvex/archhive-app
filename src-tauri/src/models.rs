use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BrowseKind {
    Tag,
    Model,
    Channel,
    Search,
    Video,
    Category,
    Livestream,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum BrowseOrientation {
    #[default]
    Straight,
    Gay,
    Lesbian,
    Transgender,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    pub id: String,
    pub title: String,
    pub url: String,
    pub thumbnail: Option<String>,
    pub duration: Option<u32>,
    pub site_id: String,
    pub performers: Vec<String>,
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_live: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewers: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embed_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowsePage {
    pub items: Vec<MediaItem>,
    pub page: u32,
    pub has_more: bool,
    pub total: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowseQuery {
    pub kind: BrowseKind,
    pub slug: String,
    pub page: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orientation: Option<BrowseOrientation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteInfo {
    pub id: String,
    pub display_name: String,
    pub base_url: String,
    pub supported_kinds: Vec<BrowseKind>,
    pub requires_cookies: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Pending,
    Active,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkImportResult {
    pub queued: u32,
    pub expanded: u32,
    pub skipped: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadJob {
    pub id: String,
    pub url: String,
    pub adapter: String,
    pub status: DownloadStatus,
    pub progress: f32,
    pub output_path: Option<String>,
    pub error: Option<String>,
    pub title: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub retry_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_retry_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadPlan {
    pub url: String,
    pub output_template: String,
    pub tool: DownloadTool,
    pub title: Option<String>,
    pub performers: Vec<String>,
    pub tags: Vec<String>,
    pub adapter_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    /// HTTP Referer to send for [`DownloadTool::DirectHttp`] downloads.
    /// Needed by Referer-gated CDNs (e.g. PornHub's phncdn).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub referer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadTool {
    YtDlp,
    GalleryDl,
    DirectHttp,
    /// HLS playlist saved to MP4 via ffmpeg stream-copy. Used when only a
    /// gated `.m3u8` exists (e.g. PornHub): ffmpeg sends Referer/Cookie
    /// headers that plain HTTP and `<video>` cannot.
    FfmpegHls,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,
    pub title: String,
    pub path: Option<String>,
    pub duration: Option<u32>,
    pub thumb: Option<String>,
    pub source_url: Option<String>,
    pub studio_id: Option<String>,
    pub studio_name: Option<String>,
    pub date: Option<String>,
    pub rating: Option<u8>,
    pub performers: Vec<String>,
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oshash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Performer {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub image: Option<String>,
    pub favorite: bool,
    pub scene_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub scene_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Studio {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EngineMode {
    Local,
    RemoteLan,
    Standalone,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SceneSort {
    #[default]
    Newest,
    Name,
    Downloaded,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DownloadQuality {
    Best,
    #[default]
    #[serde(rename = "1080")]
    Height1080,
    #[serde(rename = "720")]
    Height720,
    #[serde(rename = "480")]
    Height480,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub engine_mode: EngineMode,
    pub library_path: String,
    pub naming_template: String,
    pub lan_enabled: bool,
    pub lan_port: u16,
    pub lan_token: Option<String>,
    pub remote_host: Option<String>,
    pub remote_token: Option<String>,
    #[serde(default = "default_phash_threshold")]
    pub phash_threshold: u8,
    #[serde(default = "default_close_to_tray")]
    pub close_to_tray: bool,
    #[serde(default = "default_minimize_to_tray")]
    pub minimize_to_tray: bool,
    #[serde(default = "default_tray_hotkey")]
    pub tray_hotkey: Option<String>,
    #[serde(default)]
    pub download_quality: DownloadQuality,
    #[serde(default = "default_prefer_mp4")]
    pub prefer_mp4: bool,
    #[serde(default = "default_lan_auth_enabled")]
    pub lan_auth_enabled: bool,
    #[serde(default = "default_auto_tag_rules")]
    pub auto_tag_rules: Vec<String>,
    #[serde(default = "default_trending_sites")]
    pub trending_sites: Vec<String>,
    #[serde(default = "default_download_max_retries")]
    pub download_max_retries: u32,
    #[serde(default = "default_download_retry_delay_seconds")]
    pub download_retry_delay_seconds: u32,
    #[serde(default = "default_theme")]
    pub theme: AppTheme,
    /// Fraction of duration after which a scene counts as watched (0.5–1.0).
    #[serde(default = "default_watched_threshold")]
    pub watched_threshold: f32,
    /// Watchlist poller interval in minutes (#19 auto-queue pairing).
    #[serde(default = "default_watch_poll_interval_mins")]
    pub watch_poll_interval_mins: u32,
}

fn default_phash_threshold() -> u8 {
    10
}

fn default_close_to_tray() -> bool {
    true
}

fn default_minimize_to_tray() -> bool {
    true
}

fn default_tray_hotkey() -> Option<String> {
    Some("Ctrl+Shift+A".to_string())
}

fn default_prefer_mp4() -> bool {
    true
}

fn default_lan_auth_enabled() -> bool {
    true
}

fn default_auto_tag_rules() -> Vec<String> {
    vec![r"(?<performer>[a-zA-Z0-9_]+)-\d+".to_string()]
}

fn default_trending_sites() -> Vec<String> {
    vec![
        "pornhub".to_string(),
        "xvideos".to_string(),
        "xhamster".to_string(),
        "youporn".to_string(),
        "xnxx".to_string(),
    ]
}

fn default_download_max_retries() -> u32 {
    2
}

fn default_download_retry_delay_seconds() -> u32 {
    30
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AppTheme {
    Dark,
    Light,
    System,
}

fn default_theme() -> AppTheme {
    AppTheme::Dark
}

fn default_watched_threshold() -> f32 {
    0.9
}

fn default_watch_poll_interval_mins() -> u32 {
    60
}

impl Default for AppSettings {
    fn default() -> Self {
        #[cfg(mobile)]
        let library_path = String::new();
        #[cfg(not(mobile))]
        let library_path = dirs::video_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default())
            .join("ArcHive")
            .to_string_lossy()
            .to_string();

        #[cfg(mobile)]
        let engine_mode = EngineMode::Local;
        #[cfg(not(mobile))]
        let engine_mode = EngineMode::Local;

        #[cfg(mobile)]
        let remote_host = Some("http://192.168.178.69:8787".to_string());
        #[cfg(not(mobile))]
        let remote_host = None;

        Self {
            engine_mode,
            library_path,
            naming_template: "{performer}/{title}.{ext}".to_string(),
            lan_enabled: false,
            lan_port: 8787,
            lan_token: None,
            remote_host,
            remote_token: None,
            phash_threshold: default_phash_threshold(),
            close_to_tray: default_close_to_tray(),
            minimize_to_tray: default_minimize_to_tray(),
            tray_hotkey: default_tray_hotkey(),
            download_quality: DownloadQuality::default(),
            prefer_mp4: default_prefer_mp4(),
            lan_auth_enabled: default_lan_auth_enabled(),
            auto_tag_rules: default_auto_tag_rules(),
            trending_sites: default_trending_sites(),
            download_max_retries: default_download_max_retries(),
            download_retry_delay_seconds: default_download_retry_delay_seconds(),
            theme: default_theme(),
            watched_threshold: default_watched_threshold(),
            watch_poll_interval_mins: default_watch_poll_interval_mins(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanHost {
    pub name: String,
    pub url: String,
    pub ip: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSceneRequest {
    pub title: Option<String>,
    pub performers: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub rename_file: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchUpdateScenesRequest {
    pub scene_ids: Vec<String>,
    pub performers_add: Option<Vec<String>>,
    pub tags_add: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PornhubCategoryEntry {
    pub name: String,
    pub slug: String,
    pub orientation: BrowseOrientation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchUpdateScenesResult {
    pub updated: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub added: u32,
    pub updated: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub scanned: u32,
    pub added: u32,
    pub updated: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub match_type: String,
    pub hash: String,
    pub scenes: Vec<Scene>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_distance: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeDuplicatesResult {
    pub removed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SceneFilter {
    #[serde(default)]
    pub missing_thumb: bool,
    #[serde(default)]
    pub missing_duration: bool,
    #[serde(default)]
    pub min_duration: Option<u32>,
    #[serde(default)]
    pub max_duration: Option<u32>,
    #[serde(default)]
    pub hash_named: bool,
    #[serde(default)]
    pub hide_watched: bool,
    #[serde(default)]
    pub performer_names: Vec<String>,
    #[serde(default)]
    pub tag_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FfmpegStatus {
    pub ffmpeg_available: bool,
    pub ffprobe_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BinaryVersions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ffmpeg_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ffprobe_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ytdlp_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gallery_dl_version: Option<String>,
}

/// One subdirectory entry for the in-app folder picker (mobile fallback
/// where the native dialog plugin cannot pick directories).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirBrowseResponse {
    pub current: String,
    pub parent: Option<String>,
    pub dirs: Vec<DirEntry>,
    pub can_write: bool,
    pub roots: Vec<DirEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearThumbsResult {
    pub cleared: u64,
}

/// Result of probing an ffmpeg/ffprobe sidecar: distinguishes "not bundled
/// in this build" from "bundled but failed to execute".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarProbe {
    pub name: String,
    pub bundled: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchProgress {
    pub scene_id: String,
    pub position_secs: f64,
    pub duration_secs: f64,
    pub watched: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkWatchedRequest {
    pub scene_ids: Vec<String>,
    pub watched: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkWatchedResult {
    pub updated: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSearch {
    pub id: String,
    pub name: String,
    pub site_id: String,
    pub kind: BrowseKind,
    pub slug: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orientation: Option<BrowseOrientation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_checked_at: Option<String>,
    pub new_count: u32,
    #[serde(default)]
    pub auto_queue: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveSearchRequest {
    pub name: String,
    pub site_id: String,
    pub kind: BrowseKind,
    pub slug: String,
    #[serde(default)]
    pub orientation: Option<BrowseOrientation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckSavedSearchResult {
    pub search_id: String,
    pub new_items: Vec<MediaItem>,
    pub new_count: u32,
    pub total: usize,
    pub checked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSavedSearchRequest {
    pub auto_queue: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchlistPollResult {
    pub checked: u32,
    pub queued: u32,
    pub errors: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchPollRun {
    pub finished_at: String,
    pub checked: u32,
    pub queued: u32,
    pub errors: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchlistStatus {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run: Option<WatchPollRun>,
    pub auto_queue_count: u32,
    pub due_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbGenResult {
    pub generated: u32,
    pub errors: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationProbeResult {
    pub probed: u32,
    pub errors: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrphanSidecar {
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryStats {
    pub scene_count: u64,
    pub performer_count: u64,
    pub tag_count: u64,
    pub total_size_bytes: u64,
    pub free_space_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub mime: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesListResponse {
    pub path: String,
    pub entries: Vec<FileEntry>,
}
