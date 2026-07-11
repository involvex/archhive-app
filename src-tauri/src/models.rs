use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BrowseKind {
    Tag,
    Model,
    Channel,
    Search,
    Video,
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
    Completed,
    Failed,
    Cancelled,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadTool {
    YtDlp,
    GalleryDl,
    DirectHttp,
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
}

fn default_phash_threshold() -> u8 {
    10
}

impl Default for AppSettings {
    fn default() -> Self {
        let library_path = dirs::video_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default())
            .join("Scrawler")
            .to_string_lossy()
            .to_string();
        Self {
            engine_mode: EngineMode::Local,
            library_path,
            naming_template: "{performer}/{title}.{ext}".to_string(),
            lan_enabled: false,
            lan_port: 8787,
            lan_token: None,
            remote_host: None,
            remote_token: None,
            phash_threshold: default_phash_threshold(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
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
