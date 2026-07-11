# Custom Sites

Add support for new tube-style sites by creating a Rust adapter.

## Steps

1. Create `src-tauri/src/sites/adapters/mysite.rs`
2. Implement the `SiteAdapter` trait:

```rust
#[async_trait]
impl SiteAdapter for MySiteAdapter {
    fn id(&self) -> &str { "mysite" }
    fn display_name(&self) -> &str { "My Site" }
    fn base_url(&self) -> &str { "https://example.com" }
    fn supported_kinds(&self) -> Vec<BrowseKind> {
        vec![BrowseKind::Tag, BrowseKind::Model]
    }
    async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> { ... }
    async fn resolve_download(&self, ctx: &SiteContext, item: &MediaItem) -> AppResult<DownloadPlan> { ... }
}
```

3. Register in `src-tauri/src/sites/registry.rs`
4. Add tests with saved HTML fixtures in `src-tauri/tests/fixtures/`

## URL Patterns

Example ThotHub routes:

- `/tags/{slug}` — tag browse
- `/models/{slug}` — model browse

## Download Strategy

Most tube sites resolve to **yt-dlp**:

```rust
DownloadPlan {
    url: item.url.clone(),
    output_template: "%(uploader)s/%(title)s.%(ext)s".into(),
    tool: DownloadTool::YtDlp,
    ...
}
```

Use `DownloadTool::GalleryDl` for image galleries (Twitter, Pinterest).

## YAML Config (future)

Planned user-defined site config:

```yaml
id: mysite
base_url: https://example.com
routes:
  - kind: tag
    pattern: "/tags/{slug}"
download_strategy: yt_dlp
```
