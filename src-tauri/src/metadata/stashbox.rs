//! StashDB / stash-box GraphQL client.
//!
//! Field and input names below are taken from the upstream stash-box schema
//! (`graphql/schema/` in `stashapp/stash-box`). The schema is strict: unknown
//! fields fail the whole query, so this client only requests fields that
//! exist there. Pure `reqwest` JSON — no Python, no CDP, no native deps, so
//! it compiles for Android (`rustls`, no OpenSSL).
//!
//! Auth: `ApiKey: <key>` header (Stash convention, see
//! `docs.stashapp.cc/api`). Keys are stored in the encrypted vault, never
//! logged.
//!
//! Fingerprints: only MD5 + OSHASH go over the wire. Our local phash is a
//! base64 thumbnail hash from a different algorithm family than Stash's
//! int64 phash, so it can never match and is deliberately excluded.

use crate::error::{AppError, AppResult};
use crate::models::{
    StashPerformerMatch, StashPerformerRef, StashSceneMatch, StashStudioRef, StashTagRef,
};
use serde_json::{json, Value};

/// Default public StashDB endpoint.
pub const STASHDB_ENDPOINT: &str = "https://stashdb.org/graphql";

/// Shared scene selection — every field here exists on stash-box `Scene`:
/// `images` (not `image`), `performers` as appearances wrapping `performer`,
/// `release_date` (StashDB removed the old `date` field; boxes that only
/// know `date` are retried via [`StashBoxClient::graphql_scenes`]).
/// Kept in one place and interpolated into each query so the field lists
/// can't drift apart (unknown fields fail the whole call server-side).
/// `concat!` can't take a const, so the full documents are built once via
/// `LazyLock` instead of per call.
const SCENE_SELECTION: &str = r#"id
    title
    release_date
    details
    duration
    images { url }
    studio { id name }
    performers { performer { id name images { url } } }
    tags { name }"#;

static FIND_BY_FINGERPRINTS_QUERY: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    format!(
        r#"query FindByFingerprints($fingerprints: [[FingerprintQueryInput!]!]!) {{
  findScenesBySceneFingerprints(fingerprints: $fingerprints) {{
    {SCENE_SELECTION}
  }}
}}
"#
    )
});

static FIND_SCENE_QUERY: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    format!(
        r#"query FindScene($id: ID!) {{
  findScene(id: $id) {{
    {SCENE_SELECTION}
  }}
}}
"#
    )
});

static SEARCH_SCENES_QUERY: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    format!(
        r#"query SearchScenes($title: String!) {{
  queryScenes(input: {{ title: $title, per_page: 10 }}) {{
    count
    scenes {{
      {SCENE_SELECTION}
    }}
  }}
}}
"#
    )
});

/// Full-text search over titles, performers and studios (what Stash's own
/// Tagger uses). `queryScenes(title:)` above only filters titles, so a
/// performer name or a tube title with extra words never matched.
static SEARCH_SCENES_TERM_QUERY: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    format!(
        r#"query SearchScenesTerm($term: String!) {{
  searchScenes(term: $term, per_page: 15) {{
    count
    scenes {{
      {SCENE_SELECTION}
    }}
  }}
}}
"#
    )
});

/// Older stash-box servers: the deprecated list form.
static SEARCH_SCENE_LEGACY_QUERY: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    format!(
        r#"query SearchSceneLegacy($term: String!) {{
  searchScene(term: $term, limit: 15) {{
    {SCENE_SELECTION}
  }}
}}
"#
    )
});

const SEARCH_PERFORMERS_QUERY: &str = r#"
query SearchPerformers($name: String!) {
  queryPerformers(input: { name: $name, per_page: 10 }) {
    count
    performers {
      id
      name
      aliases
      images { url }
    }
  }
}
"#;

const SUBMIT_FINGERPRINT_MUTATION: &str = r#"
mutation SubmitFingerprint($input: FingerprintSubmission!) {
  submitFingerprint(input: $input)
}
"#;

/// One shared HTTP client for all stash-box calls: cloning is cheap and
/// shares the connection pool (keep-alive + TLS resumption). Building a new
/// `reqwest::Client` per query — e.g. once per batch Identify row — would
/// throw the pool away every time.
static SHARED_HTTP: std::sync::LazyLock<reqwest::Client> = std::sync::LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent("ArcHive/1.0 (stash-box client)")
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .expect("stash-box http client builds with static config")
});

#[derive(Clone)]
pub struct StashBoxClient {
    endpoint: String,
    endpoint_id: String,
    api_key: String,
    http: reqwest::Client,
}

impl StashBoxClient {
    pub fn new(endpoint: &str, endpoint_id: &str, api_key: &str) -> AppResult<Self> {
        let endpoint = normalize_endpoint(endpoint)?;
        if api_key.trim().is_empty() {
            return Err(AppError::InvalidInput(
                "stash-box API key is required".into(),
            ));
        }
        Ok(Self {
            endpoint,
            endpoint_id: endpoint_id.to_string(),
            api_key: api_key.to_string(),
            http: SHARED_HTTP.clone(),
        })
    }

    async fn graphql(&self, query: &str, variables: Value) -> AppResult<Value> {
        let res = self
            .http
            .post(&self.endpoint)
            .header("ApiKey", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&json!({ "query": query, "variables": variables }))
            .send()
            .await?;
        if res.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(AppError::InvalidInput(
                "stash-box rejected the API key (401). Check the key in Settings.".into(),
            ));
        }
        if !res.status().is_success() {
            return Err(AppError::Site(format!(
                "stash-box HTTP {} for {}",
                res.status(),
                self.endpoint
            )));
        }
        let body: Value = res.json().await?;
        if let Some(errors) = body.get("errors").and_then(|e| e.as_array()) {
            if !errors.is_empty() {
                let msg = errors
                    .iter()
                    .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
                    .collect::<Vec<_>>()
                    .join("; ");
                // StashDB reports a missing/invalid key as a GraphQL error
                // with HTTP 200, not a 401.
                if msg.to_ascii_lowercase().contains("not authorized") {
                    return Err(AppError::InvalidInput(format!(
                        "stash-box rejected the request as not authorized — check the API key \
                         for {} in Settings → Metadata",
                        self.endpoint
                    )));
                }
                return Err(AppError::Site(format!(
                    "stash-box GraphQL error: {}",
                    if msg.is_empty() {
                        "unknown error"
                    } else {
                        &msg
                    }
                )));
            }
        }
        Ok(body.get("data").cloned().unwrap_or(Value::Null))
    }

    /// Scene queries select `release_date`; older stash-box versions only
    /// have `date`, so retry once with that field when the box rejects it.
    async fn graphql_scenes(&self, query: &str, variables: Value) -> AppResult<Value> {
        match self.graphql(query, variables.clone()).await {
            Err(AppError::Site(msg)) if msg.contains("release_date") => {
                self.graphql(&query.replace("release_date", "date"), variables)
                    .await
            }
            other => other,
        }
    }

    /// Test credentials with a cheap query. Returns box name when known.
    pub async fn test_connection(&self) -> AppResult<String> {
        let data = self.graphql("query Me { me { name } }", json!({})).await?;
        let name = data
            .get("me")
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("stash-box")
            .to_string();
        Ok(name)
    }

    /// Find candidates by file fingerprints (MD5 + OSHASH with algorithms).
    /// One inner group per call (single scene); the server returns one result
    /// group per inner array, so flatten them all.
    pub async fn find_by_fingerprints(
        &self,
        md5: Option<&str>,
        oshash: Option<&str>,
    ) -> AppResult<Vec<StashSceneMatch>> {
        let inputs = fingerprint_inputs(md5, oshash);
        if inputs.is_empty() {
            return Err(AppError::InvalidInput(
                "scene has no MD5 or OSHASH — compute file hashes first".into(),
            ));
        }
        let data = self
            .graphql_scenes(
                FIND_BY_FINGERPRINTS_QUERY.as_str(),
                json!({ "fingerprints": [inputs] }),
            )
            .await?;
        // Response is [[Scene]] (one group per inner fingerprint array).
        let raw = data
            .get("findScenesBySceneFingerprints")
            .cloned()
            .unwrap_or(Value::Null);
        let mut out = Vec::new();
        flatten_scene_values(&raw, &mut out);
        for m in &mut out {
            m.endpoint_id = self.endpoint_id.clone();
        }
        dedupe_scenes(&mut out);
        Ok(out)
    }

    /// Direct lookup by stash id. Used by Apply so a match picked from a
    /// title search resolves exactly, instead of re-searching by filename
    /// title (which may not return the same candidate).
    pub async fn find_scene(&self, stash_id: &str) -> AppResult<Option<StashSceneMatch>> {
        let stash_id = stash_id.trim();
        if stash_id.is_empty() {
            return Err(AppError::InvalidInput("stash_id is required".into()));
        }
        let data = self
            .graphql_scenes(FIND_SCENE_QUERY.as_str(), json!({ "id": stash_id }))
            .await?;
        let raw = data.get("findScene").cloned().unwrap_or(Value::Null);
        if raw.is_null() {
            return Ok(None);
        }
        let mut m = parse_scene_value(&raw)
            .ok_or_else(|| AppError::Site("stash-box returned an unparseable scene".into()))?;
        m.endpoint_id = self.endpoint_id.clone();
        Ok(Some(m))
    }

    /// Text search when no fingerprints match: titles, performers and
    /// studios (`searchScenes`), falling back to the deprecated
    /// `searchScene` and then the title-only `queryScenes` filter on boxes
    /// that lack them.
    pub async fn search_scenes(&self, title: &str) -> AppResult<Vec<StashSceneMatch>> {
        let term = search_term(title);
        if term.is_empty() {
            return Err(AppError::InvalidInput("title is required".into()));
        }
        let vars = json!({ "term": term });
        let raw = match self
            .graphql_scenes(SEARCH_SCENES_TERM_QUERY.as_str(), vars.clone())
            .await
        {
            Ok(data) => data
                .get("searchScenes")
                .and_then(|q| q.get("scenes"))
                .cloned()
                .unwrap_or(Value::Null),
            Err(AppError::Site(msg)) if msg.contains("searchScenes") => {
                match self
                    .graphql_scenes(SEARCH_SCENE_LEGACY_QUERY.as_str(), vars)
                    .await
                {
                    Ok(data) => data.get("searchScene").cloned().unwrap_or(Value::Null),
                    Err(AppError::Site(msg)) if msg.contains("searchScene") => self
                        .graphql_scenes(SEARCH_SCENES_QUERY.as_str(), json!({ "title": term }))
                        .await?
                        .get("queryScenes")
                        .and_then(|q| q.get("scenes"))
                        .cloned()
                        .unwrap_or(Value::Null),
                    Err(e) => return Err(e),
                }
            }
            Err(e) => return Err(e),
        };
        let mut out = Vec::new();
        flatten_scene_values(&raw, &mut out);
        for m in &mut out {
            m.endpoint_id = self.endpoint_id.clone();
        }
        dedupe_scenes(&mut out);
        Ok(out)
    }

    pub async fn search_performers(&self, name: &str) -> AppResult<Vec<StashPerformerMatch>> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::InvalidInput("name is required".into()));
        }
        let data = self
            .graphql(SEARCH_PERFORMERS_QUERY, json!({ "name": name }))
            .await?;
        let raw = data
            .get("queryPerformers")
            .and_then(|q| q.get("performers"))
            .cloned()
            .unwrap_or(Value::Null);
        let mut out = Vec::new();
        if let Some(arr) = raw.as_array() {
            for v in arr {
                if let Some(m) = parse_performer_value(v) {
                    out.push(m);
                }
            }
        } else if let Some(m) = parse_performer_value(&raw) {
            out.push(m);
        }
        for m in &mut out {
            m.endpoint_id = self.endpoint_id.clone();
        }
        Ok(out)
    }

    /// Contribute this file's fingerprints back to the box (helps others match).
    /// One `submitFingerprint` call per hash; returns successful submissions.
    pub async fn submit_fingerprints(
        &self,
        scene_stash_id: &str,
        md5: Option<&str>,
        oshash: Option<&str>,
        duration_secs: Option<u32>,
    ) -> AppResult<u32> {
        let duration = duration_secs.unwrap_or(0);
        if duration == 0 {
            return Err(AppError::InvalidInput(
                "scene duration is unknown — probe metadata first".into(),
            ));
        }
        let inputs = fingerprint_inputs(md5, oshash);
        if inputs.is_empty() {
            return Err(AppError::InvalidInput(
                "scene has no fingerprints to submit".into(),
            ));
        }
        let mut submitted = 0u32;
        for fp in &inputs {
            let data = self
                .graphql(
                    SUBMIT_FINGERPRINT_MUTATION,
                    json!({ "input": {
                        "scene_id": scene_stash_id,
                        "fingerprint": {
                            "hash": fp.get("hash"),
                            "algorithm": fp.get("algorithm"),
                            "duration": duration as i64,
                        },
                    }}),
                )
                .await?;
            if data
                .get("submitFingerprint")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                submitted += 1;
            }
        }
        Ok(submitted)
    }
}

/// `FingerprintQueryInput` rows (`{ hash, algorithm }`) for the hashes worth
/// sending. Our thumbnail phash is excluded: Stash stores int64 phashes and
/// ours (base64, different algorithm) could never match.
/// Tube titles → a search term: emoji and decoration dropped, words kept
/// ("🔥❤️SisLovesMe - Lacy Lennon Fucks Her Stepbro" →
/// "SisLovesMe Lacy Lennon Fucks Her Stepbro").
fn search_term(title: &str) -> String {
    title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '\'' || c == '.' {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn fingerprint_inputs(md5: Option<&str>, oshash: Option<&str>) -> Vec<Value> {
    let mut inputs = Vec::new();
    if let Some(h) = md5.filter(|h| !h.trim().is_empty()) {
        inputs.push(json!({ "hash": h.trim(), "algorithm": "MD5" }));
    }
    if let Some(h) = oshash.filter(|h| !h.trim().is_empty()) {
        inputs.push(json!({ "hash": h.trim(), "algorithm": "OSHASH" }));
    }
    inputs
}

/// Require http(s) GraphQL URL. Rejects `javascript:`, `file:`, bare hosts.
pub fn normalize_endpoint(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput("endpoint is required".into()));
    }
    let lower = trimmed.to_lowercase();
    if !(lower.starts_with("https://") || lower.starts_with("http://")) {
        return Err(AppError::InvalidInput(
            "endpoint must be an http(s) URL (e.g. https://stashdb.org/graphql)".into(),
        ));
    }
    if trimmed.len() > 256 {
        return Err(AppError::InvalidInput("endpoint URL too long".into()));
    }
    Ok(trimmed.to_string())
}

fn dedupe_scenes(scenes: &mut Vec<StashSceneMatch>) {
    let mut seen = std::collections::HashSet::new();
    scenes.retain(|s| seen.insert(s.stash_id.clone()));
}

fn flatten_scene_values(raw: &Value, out: &mut Vec<StashSceneMatch>) {
    match raw {
        Value::Array(arr) => {
            for v in arr {
                if v.is_array() {
                    flatten_scene_values(v, out);
                } else if let Some(m) = parse_scene_value(v) {
                    out.push(m);
                }
            }
        }
        Value::Object(_) => {
            if let Some(m) = parse_scene_value(raw) {
                out.push(m);
            }
        }
        _ => {}
    }
}

fn str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
}

/// First `images[].url` — stash-box has no singular `image` field.
fn first_image_url(v: &Value) -> Option<String> {
    v.get("images")?.as_array()?.iter().find_map(|img| {
        img.get("url")?
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.to_string())
    })
}

fn parse_scene_value(v: &Value) -> Option<StashSceneMatch> {
    let stash_id = str_field(v, "id")?;
    let title = str_field(v, "title").unwrap_or_else(|| "Untitled".to_string());
    // `date` is deprecated upstream in favor of `release_date`; accept both.
    let date = str_field(v, "release_date").or_else(|| str_field(v, "date"));
    let studio = v.get("studio").and_then(|s| {
        if s.is_null() {
            return None;
        }
        let name = str_field(s, "name")?;
        Some(StashStudioRef {
            name,
            stash_id: str_field(s, "id"),
            image: first_image_url(s),
        })
    });
    let performers = v
        .get("performers")
        .and_then(|p| p.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|appearance| {
                    // PerformerAppearance wraps the performer object.
                    let p = appearance.get("performer").unwrap_or(appearance);
                    Some(StashPerformerRef {
                        name: str_field(p, "name")?,
                        stash_id: str_field(p, "id"),
                        image: first_image_url(p),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let tags = v
        .get("tags")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    // Boxes return `{ name }` or `{ tag: { name } }`.
                    let name = str_field(t, "name")
                        .or_else(|| t.get("tag").and_then(|inner| str_field(inner, "name")))?;
                    Some(StashTagRef { name })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(StashSceneMatch {
        stash_id,
        title,
        date,
        details: str_field(v, "details"),
        studio,
        performers,
        tags,
        image: first_image_url(v),
        duration: v.get("duration").and_then(|d| d.as_u64()).map(|d| d as u32),
        endpoint_id: String::new(),
        ..Default::default()
    })
}

fn parse_performer_value(v: &Value) -> Option<StashPerformerMatch> {
    let stash_id = str_field(v, "id")?;
    let name = str_field(v, "name")?;
    let aliases = v
        .get("aliases")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    Some(StashPerformerMatch {
        stash_id,
        name,
        aliases,
        image: first_image_url(v),
        endpoint_id: String::new(),
    })
}

#[cfg(test)]
mod tests {

    #[test]
    fn search_term_strips_decoration() {
        assert_eq!(
            search_term("🔥❤️SisLovesMe - Lacy Lennon Fucks Her Stepbro"),
            "SisLovesMe Lacy Lennon Fucks Her Stepbro"
        );
        assert_eq!(search_term("  Lacy Lennon "), "Lacy Lennon");
        assert_eq!(search_term("🔥❤️"), "");
    }

    use super::*;

    // Keeps the shared selection pasted into each query in sync: every query
    // must request exactly these fields (unknown fields fail the whole call).
    #[test]
    fn all_queries_use_schema_fields_only() {
        for query in [
            FIND_BY_FINGERPRINTS_QUERY.as_str(),
            FIND_SCENE_QUERY.as_str(),
            SEARCH_SCENES_QUERY.as_str(),
            SEARCH_PERFORMERS_QUERY,
        ] {
            assert!(
                !query.contains("queryScenesByFingerprints"),
                "removed upstream: {query}"
            );
            // No singular `image` selections (stash-box uses `images { url }`).
            for line in query.lines() {
                let field = line.trim();
                assert!(
                    !field.starts_with("image ") && field != "image",
                    "singular image field: {field}"
                );
            }
        }
        assert!(FIND_BY_FINGERPRINTS_QUERY.contains("findScenesBySceneFingerprints"));
        assert!(FIND_BY_FINGERPRINTS_QUERY.contains("FingerprintQueryInput"));
        assert!(FIND_SCENE_QUERY.contains("findScene"));
        // All scene queries share one selection — same fields, no drift.
        for query in [
            FIND_BY_FINGERPRINTS_QUERY.as_str(),
            FIND_SCENE_QUERY.as_str(),
            SEARCH_SCENES_QUERY.as_str(),
        ] {
            assert!(query.contains(SCENE_SELECTION));
        }
        assert!(SEARCH_SCENES_QUERY.contains("queryScenes"));
        assert!(SEARCH_PERFORMERS_QUERY.contains("queryPerformers"));
        assert!(SUBMIT_FINGERPRINT_MUTATION.contains("submitFingerprint"));
        assert!(!SUBMIT_FINGERPRINT_MUTATION.contains("submitSceneFingerprints"));
    }

    #[test]
    fn fingerprint_inputs_carry_algorithms() {
        let inputs = fingerprint_inputs(Some("abc123"), Some("0011223344556677"));
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0]["algorithm"], json!("MD5"));
        assert_eq!(inputs[1]["algorithm"], json!("OSHASH"));
        assert!(fingerprint_inputs(None, Some("  ")).is_empty());
        assert!(fingerprint_inputs(None, None).is_empty());
    }

    #[test]
    fn rejects_non_http_endpoints() {
        assert!(normalize_endpoint("https://stashdb.org/graphql").is_ok());
        assert!(normalize_endpoint("http://192.168.1.2:9999/graphql").is_ok());
        assert!(normalize_endpoint("javascript:alert(1)").is_err());
        assert!(normalize_endpoint("stashdb.org/graphql").is_err());
        assert!(normalize_endpoint("").is_err());
    }

    #[test]
    fn parses_scene_search_response() {
        let v: Value = serde_json::json!({
            "id": "abc-123",
            "title": "Test Scene",
            "release_date": "2024-01-02",
            "details": "Some details",
            "duration": 1800,
            "images": [{ "url": "https://example.com/cover.jpg" }],
            "studio": { "id": "s1", "name": "Studio One" },
            "performers": [{ "performer": { "id": "p1", "name": "Jane Doe" } }],
            "tags": [{ "name": "tag-a" }, { "tag": { "name": "tag-b" } }],
        });
        let m = parse_scene_value(&v).expect("parses");
        assert_eq!(m.stash_id, "abc-123");
        assert_eq!(m.date.as_deref(), Some("2024-01-02"));
        assert_eq!(m.image.as_deref(), Some("https://example.com/cover.jpg"));
        assert_eq!(m.performers.len(), 1);
        assert_eq!(m.performers[0].name, "Jane Doe");
        assert_eq!(m.tags.len(), 2);
        assert_eq!(m.studio.unwrap().name, "Studio One");
    }

    #[test]
    fn flattens_grouped_fingerprint_results_and_dedupes() {
        // Server returns one group per inner fingerprint array.
        let raw: Value = serde_json::json!([
            [{ "id": "a", "title": "A" }],
            [{ "id": "a", "title": "A" }, { "id": "b", "title": "B" }],
        ]);
        let mut out = Vec::new();
        flatten_scene_values(&raw, &mut out);
        dedupe_scenes(&mut out);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn parses_performer_search_response() {
        let v: Value = serde_json::json!({
            "id": "p1",
            "name": "Jane",
            "aliases": ["J"],
            "images": [{ "url": "https://example.com/j.jpg" }],
        });
        let m = parse_performer_value(&v).expect("parses");
        assert_eq!(m.aliases, vec!["J".to_string()]);
        assert_eq!(m.image.as_deref(), Some("https://example.com/j.jpg"));
    }
}
