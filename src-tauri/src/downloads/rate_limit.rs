//! Global yt-dlp `--limit-rate` token helpers.

/// Normalize a user-supplied rate string for yt-dlp `--limit-rate`.
///
/// Accepts empty / whitespace (unlimited → `None`), or a simple rate token
/// such as `500K`, `2M`, `1.5G`, or a plain integer byte rate. Rejects
/// anything that could be unsafe to pass as a Command argument.
pub fn normalize_rate_limit(raw: Option<&str>) -> Result<Option<String>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if !is_valid_rate_token(trimmed) {
        return Err(format!(
            "invalid download rate limit '{trimmed}'; use e.g. 500K, 2M, or leave blank"
        ));
    }
    Ok(Some(trimmed.to_string()))
}

/// True for yt-dlp-style rate tokens: optional decimal + optional K/M/G/T suffix.
pub fn is_valid_rate_token(token: &str) -> bool {
    let bytes = token.as_bytes();
    if bytes.is_empty() || bytes.len() > 16 {
        return false;
    }
    let (num, suffix) = match bytes.last().map(|b| b.to_ascii_uppercase()) {
        Some(b @ (b'K' | b'M' | b'G' | b'T')) => (&bytes[..bytes.len() - 1], Some(b)),
        _ => (bytes, None),
    };
    if num.is_empty() {
        return false;
    }
    let mut saw_digit = false;
    let mut saw_dot = false;
    for &b in num {
        match b {
            b'0'..=b'9' => saw_digit = true,
            b'.' if !saw_dot => saw_dot = true,
            _ => return false,
        }
    }
    if !saw_digit {
        return false;
    }
    // Suffix is optional; presence already validated above.
    let _ = suffix;
    true
}

/// Append `--limit-rate <token>` when a normalized rate is set.
pub fn append_limit_rate_args(args: &mut Vec<String>, rate: Option<&str>) {
    if let Some(token) = rate.filter(|s| !s.is_empty()) {
        args.push("--limit-rate".to_string());
        args.push(token.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_unlimited() {
        assert_eq!(normalize_rate_limit(None).unwrap(), None);
        assert_eq!(normalize_rate_limit(Some("")).unwrap(), None);
        assert_eq!(normalize_rate_limit(Some("  ")).unwrap(), None);
    }

    #[test]
    fn accepts_common_tokens() {
        assert_eq!(
            normalize_rate_limit(Some("500K")).unwrap(),
            Some("500K".into())
        );
        assert_eq!(
            normalize_rate_limit(Some("2M")).unwrap(),
            Some("2M".into())
        );
        assert_eq!(
            normalize_rate_limit(Some("1.5G")).unwrap(),
            Some("1.5G".into())
        );
        assert_eq!(
            normalize_rate_limit(Some("1024")).unwrap(),
            Some("1024".into())
        );
        assert!(is_valid_rate_token("50k"));
        assert!(is_valid_rate_token("2t"));
    }

    #[test]
    fn rejects_injection_and_junk() {
        assert!(normalize_rate_limit(Some("500K; rm -rf /")).is_err());
        assert!(normalize_rate_limit(Some("--help")).is_err());
        assert!(normalize_rate_limit(Some("2 M")).is_err());
        assert!(normalize_rate_limit(Some("../x")).is_err());
        assert!(normalize_rate_limit(Some("K")).is_err());
        assert!(!is_valid_rate_token(""));
    }

    #[test]
    fn append_args_only_when_set() {
        let mut args = vec!["url".into()];
        append_limit_rate_args(&mut args, None);
        assert_eq!(args, vec!["url"]);
        append_limit_rate_args(&mut args, Some("2M"));
        assert_eq!(args, vec!["url", "--limit-rate", "2M"]);
    }
}
