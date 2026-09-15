use crate::error::AppResult;
use crate::sites::SiteContext;
use regex::Regex;
use std::sync::OnceLock;

/// Best-effort direct MP4 extraction for ThotHub / Kernel Video Sharing pages.
///
/// KVS players embed `video_url` / `video_alt_url*` (and `license_code`) in a
/// page script. Newer builds already expose plain `/get_file/...` HTTPS URLs;
/// older ones prefix obfuscated URLs with `function/0/` which we decode using
/// the same license-token swap as yt-dlp's generic KVS extractor.
///
/// Returns `Ok(None)` when nothing playable is found so callers can fall back
/// to yt-dlp (desktop) or surface a clear error (mobile).
pub async fn extract_stream_url(ctx: &SiteContext, url: &str) -> AppResult<Option<String>> {
    let html = ctx.fetch_html(url, "thothub").await?;
    Ok(pick_best_stream(&html, url))
}

fn pick_best_stream(html: &str, page_url: &str) -> Option<String> {
    let license = extract_license_code(html);
    let mut candidates = collect_video_urls(html);
    if candidates.is_empty() {
        if let Some(u) = extract_json_ld_content_url(html) {
            candidates.push((0, u));
        }
    }
    if candidates.is_empty() {
        return None;
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    let raw = &candidates[0].1;
    let decoded = match &license {
        Some(code) => kvs_get_real_url(raw, code),
        None => raw.clone(),
    };
    Some(absolutize(page_url, &decoded))
}

fn extract_license_code(html: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r#"license_code\s*:\s*['"]([^'"]+)['"]"#).expect("license regex")
    });
    re.captures(html)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

fn collect_video_urls(html: &str) -> Vec<(i32, String)> {
    static URL_RE: OnceLock<Regex> = OnceLock::new();
    let url_re = URL_RE.get_or_init(|| {
        Regex::new(r#"(video_url|video_alt_url\d*)\s*:\s*['"]([^'"]+)['"]"#)
            .expect("url key regex")
    });
    static TEXT_RE: OnceLock<Regex> = OnceLock::new();
    let text_re = TEXT_RE.get_or_init(|| {
        Regex::new(r#"(video_url|video_alt_url\d*)_text\s*:\s*['"]([^'"]+)['"]"#)
            .expect("text key regex")
    });

    let mut texts: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for cap in text_re.captures_iter(html) {
        texts.insert(cap[1].to_string(), cap[2].to_string());
    }

    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for cap in url_re.captures_iter(html) {
        let key = cap[1].to_string();
        let url = cap[2].to_string();
        if !url.contains("/get_file/") && !url.contains("function/0/") {
            continue;
        }
        if !seen.insert(url.clone()) {
            continue;
        }
        let height = texts
            .get(&key)
            .map(|t| parse_height(t))
            .unwrap_or_else(|| parse_height(&url));
        out.push((height, url));
    }
    out
}

fn extract_json_ld_content_url(html: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r#""contentUrl"\s*:\s*"([^"]+get_file[^"]+)""#).expect("jsonld regex")
    });
    re.captures(html)
        .and_then(|c| c.get(1).map(|m| m.as_str().replace("\\/", "/")))
}

fn parse_height(s: &str) -> i32 {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(\d{3,4})p").expect("height regex"));
    re.captures(s)
        .and_then(|c| c[1].parse().ok())
        .unwrap_or(0)
}

fn absolutize(page_url: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }
    if let Ok(base) = url::Url::parse(page_url) {
        if let Ok(joined) = base.join(href) {
            return joined.to_string();
        }
    }
    href.to_string()
}

/// Port of yt-dlp `GenericIE._kvs_get_real_url`.
fn kvs_get_real_url(video_url: &str, license_code: &str) -> String {
    const PREFIX: &str = "function/0/";
    if !video_url.starts_with(PREFIX) {
        return video_url.to_string();
    }
    let rest = &video_url[PREFIX.len()..];
    let Ok(parsed) = url::Url::parse(rest) else {
        return video_url.to_string();
    };
    let license_token = kvs_get_license_token(license_code);
    let path = parsed.path();
    let mut urlparts: Vec<String> = path.split('/').map(|s| s.to_string()).collect();
    const HASH_LENGTH: usize = 32;
    if urlparts.len() <= 3 || urlparts[3].len() < HASH_LENGTH {
        return rest.to_string();
    }
    let hash = urlparts[3][..HASH_LENGTH].to_string();
    let mut indices: Vec<usize> = (0..HASH_LENGTH).collect();
    let mut accum: usize = 0;
    for src in (0..HASH_LENGTH).rev() {
        accum += license_token.get(src).copied().unwrap_or(0) as usize;
        let dest = (src + accum) % HASH_LENGTH;
        indices.swap(src, dest);
    }
    let hash_chars: Vec<char> = hash.chars().collect();
    let mut new_hash = String::with_capacity(urlparts[3].len());
    for index in indices {
        new_hash.push(hash_chars.get(index).copied().unwrap_or('0'));
    }
    new_hash.push_str(&urlparts[3][HASH_LENGTH..]);
    urlparts[3] = new_hash;
    let mut out = parsed;
    out.set_path(&urlparts.join("/"));
    out.to_string()
}

fn kvs_get_license_token(license_code: &str) -> Vec<u32> {
    let license_code = license_code.replace('$', "");
    let license_values: Vec<u32> = license_code
        .chars()
        .filter_map(|c| c.to_digit(10))
        .collect();
    let modlicense = license_code.replace('0', "1");
    let center = modlicense.len() / 2;
    let fronthalf: i64 = modlicense[..=center].parse().unwrap_or(0);
    let backhalf: i64 = modlicense[center..].parse().unwrap_or(0);
    let mod_str = (4 * (fronthalf - backhalf).abs()).to_string();
    let mod_trim: String = mod_str.chars().take(center + 1).collect();
    let mut token = Vec::new();
    for (index, current) in mod_trim.chars().filter_map(|c| c.to_digit(10)).enumerate() {
        for offset in 0..4 {
            let lv = license_values.get(index + offset).copied().unwrap_or(0);
            token.push((lv + current) % 10);
        }
    }
    token
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"
<script>
var playercfg = {
  video_id: '105',
  license_code: '$496544316381867',
  video_url: 'https://www.kvs-demo.com/get_file/1/abc/0/105/105_480p.mp4/',
  video_url_text: '480p',
  video_alt_url: 'https://www.kvs-demo.com/get_file/1/def/0/105/105_720p.mp4/',
  video_alt_url_text: '720p',
};
</script>
<script type="application/ld+json">
{"contentUrl": "https://www.kvs-demo.com/get_file/1/f2aded36c9fc0a2c1f3653b6a2741361/0/105/105_720p.mp4/"}
</script>
"#;

    #[test]
    fn picks_highest_quality_get_file_url() {
        let url = pick_best_stream(FIXTURE, "https://www.kvs-demo.com/video/105/x/").unwrap();
        assert!(url.contains("105_720p.mp4"), "got {url}");
    }

    #[test]
    fn deobfuscates_function0_prefix() {
        // Path layout: /get_file/1/{32-char-hash}rest/file.mp4
        let obfuscated = "function/0/https://cdn.example.com/get_file/1/0123456789abcdef0123456789abcdefREST/file.mp4";
        let license = "$496544316381867";
        let real = kvs_get_real_url(obfuscated, license);
        assert!(!real.starts_with("function/0/"), "{real}");
        assert!(real.contains("cdn.example.com/get_file/1/"), "{real}");
        assert!(
            real.ends_with("REST/file.mp4") || real.contains("REST/file.mp4"),
            "{real}"
        );
        let parsed = url::Url::parse(&real).unwrap();
        let parts: Vec<_> = parsed.path().split('/').collect();
        assert!(parts[3].len() >= 32);
        assert_eq!(
            parts[3].len(),
            "0123456789abcdef0123456789abcdefREST".len()
        );
    }

    #[test]
    fn plain_url_passthrough() {
        let u = "https://cdn.example.com/get_file/1/hash/file.mp4";
        assert_eq!(kvs_get_real_url(u, "$123"), u);
    }

    #[test]
    fn parses_real_kvs_player_snippet() {
        let html = std::fs::read_to_string("tests/fixtures/kvs_player_snippet.html")
            .expect("fixture kvs_player_snippet.html");
        let url = pick_best_stream(&html, "https://www.kvs-demo.com/video/105/x/").unwrap();
        assert!(
            url.contains("/get_file/") && url.contains("720p"),
            "expected 720p get_file url, got {url}"
        );
    }
}
