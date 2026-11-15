use regex::Regex;

pub fn apply_filename_rules(filename: &str, rules: &[String]) -> (Vec<String>, Vec<String>) {
    let mut performers = Vec::new();
    let mut tags = Vec::new();

    for rule in rules {
        if let Ok(re) = Regex::new(rule) {
            if let Some(caps) = re.captures(filename) {
                if let Some(p) = caps.name("performer") {
                    performers.push(p.as_str().to_string());
                }
                if let Some(t) = caps.name("tag") {
                    tags.push(t.as_str().to_string());
                }
            }
        }
    }

    if performers.is_empty() {
        if let Some((performer, _)) = filename.split_once('-') {
            let p = performer.trim().to_string();
            if !p.is_empty() && p.len() < 40 {
                performers.push(p);
            }
        }
    }

    (performers, tags)
}

/// Suggest tags (and optional performers) the user can confirm-to-apply.
/// Based on filename rules + simple title token heuristics; never overwrites.
pub fn suggest_from_title_and_path(
    title: &str,
    path: Option<&str>,
    rules: &[String],
    existing_tags: &[String],
    existing_performers: &[String],
) -> (Vec<String>, Vec<String>) {
    let stem = path
        .and_then(|p| std::path::Path::new(p).file_stem())
        .and_then(|s| s.to_str())
        .unwrap_or(title);
    let (fn_performers, fn_tags) = apply_filename_rules(stem, rules);

    let mut tags = Vec::new();
    for t in fn_tags {
        if !existing_tags.iter().any(|e| e.eq_ignore_ascii_case(&t))
            && !tags.iter().any(|e: &String| e.eq_ignore_ascii_case(&t))
        {
            tags.push(t);
        }
    }
    // Title tokens like `[tag]` or `#tag`.
    for part in title.split(|c: char| c.is_whitespace() || c == '_' || c == '.') {
        let cleaned = part.trim_matches(|c: char| {
            c == '#' || c == '[' || c == ']' || c == '(' || c == ')' || c == ','
        });
        if cleaned.len() < 3 || cleaned.len() > 32 {
            continue;
        }
        if !cleaned.chars().any(|c| c.is_ascii_alphabetic()) {
            continue;
        }
        if cleaned.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if existing_tags
            .iter()
            .any(|e| e.eq_ignore_ascii_case(cleaned))
            || tags.iter().any(|e: &String| e.eq_ignore_ascii_case(cleaned))
        {
            continue;
        }
        // Only keep hashtag / bracket style as tag suggestions from title.
        if part.starts_with('#') || part.starts_with('[') {
            tags.push(cleaned.to_string());
        }
        if tags.len() >= 12 {
            break;
        }
    }

    let mut performers = Vec::new();
    for p in fn_performers {
        if !existing_performers
            .iter()
            .any(|e| e.eq_ignore_ascii_case(&p))
            && !performers.iter().any(|e: &String| e.eq_ignore_ascii_case(&p))
        {
            performers.push(p);
        }
    }
    (performers, tags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_performer_from_rule() {
        let rules = vec![r"(?<performer>[a-z]+)-\d+".to_string()];
        let (p, _) = apply_filename_rules("leynainu-12345", &rules);
        assert_eq!(p, vec!["leynainu"]);
    }

    #[test]
    fn suggest_skips_existing_and_picks_hashtags() {
        let (perf, tags) = suggest_from_title_and_path(
            "Clip title #outdoor [hd]",
            Some("/lib/alice-123.mp4"),
            &[r"(?<performer>[a-z]+)-\d+".to_string()],
            &["hd".into()],
            &[],
        );
        assert!(perf.iter().any(|p| p == "alice"));
        assert!(tags.iter().any(|t| t == "outdoor"));
        assert!(!tags.iter().any(|t| t.eq_ignore_ascii_case("hd")));
    }
}
