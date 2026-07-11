use crate::error::{AppError, AppResult};
use crate::models::{BrowseKind, BrowsePage, BrowseQuery, DownloadPlan, DownloadTool, MediaItem};
use crate::sites::{SiteAdapter, SiteContext};
use async_trait::async_trait;
use scraper::{Html, Selector};
use uuid::Uuid;

pub struct ThotHubAdapter;

const BASE: &str = "https://thethothub.com";

#[async_trait]
impl SiteAdapter for ThotHubAdapter {
    fn id(&self) -> &str {
        "thothub"
    }

    fn display_name(&self) -> &str {
        "ThotHub"
    }

    fn base_url(&self) -> &str {
        BASE
    }

    fn supported_kinds(&self) -> Vec<BrowseKind> {
        vec![BrowseKind::Tag, BrowseKind::Model, BrowseKind::Search, BrowseKind::Video]
    }

    async fn browse(&self, ctx: &SiteContext, query: BrowseQuery) -> AppResult<BrowsePage> {
        let url = build_browse_url(&query)?;
        let html = ctx.fetch_html(&url, self.id()).await?;
        let items = parse_listing(&html, self.id())?;
        let has_more = items.len() >= 24;
        Ok(BrowsePage {
            items,
            page: query.page,
            has_more,
            total: None,
        })
    }

    async fn resolve_download(&self, _ctx: &SiteContext, item: &MediaItem) -> AppResult<DownloadPlan> {
        Ok(DownloadPlan {
            url: item.url.clone(),
            output_template: "%(uploader)s/%(title)s.%(ext)s".to_string(),
            tool: DownloadTool::YtDlp,
            title: Some(item.title.clone()),
            performers: item.performers.clone(),
            tags: item.tags.clone(),
            adapter_id: self.id().to_string(),
        })
    }
}

fn build_browse_url(query: &BrowseQuery) -> AppResult<String> {
    let path = match query.kind {
        BrowseKind::Tag => format!("/tags/{}/", query.slug),
        BrowseKind::Model => format!("/models/{}/", query.slug),
        BrowseKind::Search => format!("/search/{}/", query.slug),
        BrowseKind::Video => {
            if query.slug.starts_with("http") {
                return Ok(query.slug.clone());
            }
            format!("/videos/{}/", query.slug)
        }
        BrowseKind::Channel => format!("/channels/{}/", query.slug),
    };
    let page_suffix = if query.page > 1 {
        format!("?page={}", query.page)
    } else {
        String::new()
    };
    Ok(format!("{BASE}{path}{page_suffix}"))
}

fn parse_listing(html: &str, site_id: &str) -> AppResult<Vec<MediaItem>> {
    let document = Html::parse_document(html);
    let item_sel = Selector::parse(".video-item, .thumb, article, .post-item").unwrap_or_else(|_| {
        Selector::parse("a").unwrap()
    });
    let link_sel = Selector::parse("a").unwrap();
    let img_sel = Selector::parse("img").unwrap();
    let title_sel = Selector::parse("a, .title, h2, h3").unwrap();

    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for el in document.select(&item_sel) {
        let link = el
            .select(&link_sel)
            .find_map(|a| a.value().attr("href"))
            .or_else(|| el.value().attr("href"));

        let Some(href) = link else { continue };
        if !href.contains("/video") && !href.contains("/watch") && !href.contains("/v/") {
            continue;
        }

        let url = if href.starts_with("http") {
            href.to_string()
        } else {
            format!("{BASE}{}", href.trim_start_matches('/').trim_start_matches(BASE))
        };

        if !seen.insert(url.clone()) {
            continue;
        }

        let title = el
            .select(&title_sel)
            .find_map(|t| {
                if let Some(attr) = t.value().attr("title") {
                    return Some(attr.to_string());
                }
                let text = t.text().collect::<String>();
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
            .unwrap_or_else(|| "Untitled".to_string());

        let thumbnail = el
            .select(&img_sel)
            .find_map(|img| {
                img.value()
                    .attr("data-src")
                    .or_else(|| img.value().attr("src"))
                    .map(|s| s.to_string())
            });

        items.push(MediaItem {
            id: Uuid::new_v4().to_string(),
            title,
            url,
            thumbnail,
            duration: None,
            site_id: site_id.to_string(),
            performers: vec![],
            tags: vec![],
        });

        if items.len() >= 48 {
            break;
        }
    }

    if items.is_empty() {
        return Err(AppError::Site(
            "No videos found on page. Site layout may have changed.".to_string(),
        ));
    }

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_tag_url() {
        let q = BrowseQuery {
            kind: BrowseKind::Tag,
            slug: "example".to_string(),
            page: 1,
        };
        assert!(build_browse_url(&q).unwrap().contains("/tags/example"));
    }
}
