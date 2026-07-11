/// Path segment: spaces to hyphens, lowercased (pornstar/model URLs).
pub fn path_slug(slug: &str) -> String {
    slug.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

/// Query string value (search terms with spaces).
pub fn query_slug(slug: &str) -> String {
    url::form_urlencoded::byte_serialize(slug.trim().as_bytes()).collect()
}
