//! Desktop counterpart of the Android in-app-browser fallback
//! (`sites::challenge`): getting past Cloudflare / bot checks that refuse
//! yt-dlp and the Rust HTTP client.
//!
//! 1. Retry yt-dlp with browser impersonation (`--impersonate`, needs the
//!    standalone yt-dlp builds that bundle curl_cffi: yt-dlp.exe,
//!    yt-dlp_macos, yt-dlp_linux). That alone passes fingerprint-based
//!    blocks.
//! 2. Still refused: open the page in a small ArcHive browser window, where
//!    the check passes by itself or after one click, then reuse its cookies
//!    (`cf_clearance`, session) with the same User-Agent. They are stored in
//!    the encrypted vault per site and tried first next time.
//!
//! The window, yt-dlp and the Rust client all send [`CLEARANCE_UA`]:
//! Cloudflare ties the clearance cookie to the User-Agent it was issued to.
#![cfg_attr(not(desktop), allow(dead_code))]

use crate::error::{AppError, AppResult};
use crate::vault::CookieVault;
use std::path::PathBuf;

/// yt-dlp impersonation target matching [`CLEARANCE_UA`].
pub const IMPERSONATE_TARGET: &str = "chrome-136:macos-15";
/// Fallback when a yt-dlp build lacks [`IMPERSONATE_TARGET`].
pub const IMPERSONATE_FALLBACK: &str = "chrome";
pub const CLEARANCE_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36";

/// Saved clearance for one site: a Netscape cookie file and the UA it is
/// valid for.
#[derive(Debug, Clone)]
pub struct Clearance {
    pub vault_key: String,
    pub cookie_file: PathBuf,
    pub user_agent: String,
}

fn host_of(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()?
        .host_str()
        .map(|h| h.trim_start_matches("www.").to_ascii_lowercase())
}

/// Vault cookie key for a site's clearance: the site's per-domain entry
/// (`host-crazyporn-xxx`), shared with cookies from Settings → Cookies.
pub fn vault_key(url: &str) -> Option<String> {
    crate::sites::cookie_keys::host_key(url)
}

fn ua_secret_key(vault_key: &str) -> String {
    format!("{vault_key}:ua")
}

/// yt-dlp arguments for impersonated requests (generic extractor included).
pub fn impersonate_args(target: &str, user_agent: Option<&str>) -> Vec<String> {
    let mut args = vec![
        "--impersonate".to_string(),
        target.to_string(),
        "--extractor-args".to_string(),
        "generic:impersonate".to_string(),
    ];
    if let Some(ua) = user_agent.filter(|u| !u.is_empty()) {
        args.push("--user-agent".to_string());
        args.push(ua.to_string());
    }
    args
}

/// The yt-dlp build can't impersonate this target (old build, or the
/// Python zipapp without curl_cffi).
pub fn impersonation_unavailable(err: &str) -> bool {
    let e = err.to_ascii_lowercase();
    e.contains("impersonate target") && (e.contains("not available") || e.contains("unavailable"))
}

/// Clearance saved earlier for this site, if any.
pub fn stored(vault: &CookieVault, url: &str) -> Option<Clearance> {
    let key = vault_key(url)?;
    let cookie_file = vault.cookie_file_for_site(&key)?;
    let user_agent = vault
        .get_secret(&ua_secret_key(&key))
        .ok()
        .flatten()
        .unwrap_or_else(|| CLEARANCE_UA.to_string());
    Some(Clearance {
        vault_key: key,
        cookie_file,
        user_agent,
    })
}

/// Open `url` in a browser window until the bot check is gone, then save
/// the window's cookies for the site.
#[cfg(desktop)]
pub async fn obtain(
    app: &tauri::AppHandle,
    vault: &CookieVault,
    url: &str,
) -> AppResult<Clearance> {
    static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _guard = LOCK.lock().await;

    let key = vault_key(url).ok_or_else(|| AppError::InvalidInput(format!("bad URL: {url}")))?;
    let host = host_of(url).unwrap_or_default();
    let cookies = crate::cookie_login::clear_bot_check(app, url, &host, CLEARANCE_UA).await?;
    if cookies.is_empty() {
        return Err(AppError::Site(format!(
            "The browser window passed {host}, but it set no cookies to reuse."
        )));
    }
    vault.save_cookies(&key, &crate::cookie_login::to_netscape(&cookies))?;
    vault.set_secret(&ua_secret_key(&key), CLEARANCE_UA)?;
    let cookie_file = vault
        .cookie_file_for_site(&key)
        .ok_or_else(|| AppError::Other("saved clearance cookies are missing".into()))?;
    Ok(Clearance {
        vault_key: key,
        cookie_file,
        user_agent: CLEARANCE_UA.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_and_args() {
        assert_eq!(
            vault_key("https://www.crazyporn.xxx/videos/1/x/").as_deref(),
            Some("host-crazyporn-xxx")
        );
        assert_eq!(vault_key("not a url"), None);
        assert_eq!(
            impersonate_args("chrome", Some("UA/1")),
            [
                "--impersonate",
                "chrome",
                "--extractor-args",
                "generic:impersonate",
                "--user-agent",
                "UA/1"
            ]
        );
        assert!(impersonation_unavailable(
            "ERROR: Impersonate target \"chrome-136:macos-15\" is not available."
        ));
        assert!(!impersonation_unavailable("HTTP Error 403: Forbidden"));
    }
}
