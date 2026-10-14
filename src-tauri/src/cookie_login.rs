//! In-app browser login that captures a site's cookies.
//!
//! Instead of exporting cookies from an external browser, the user logs in
//! inside a browser ArcHive controls and the resulting cookies (including
//! HttpOnly session cookies that page JavaScript cannot see) are converted to
//! Netscape format for the cookie vault / yt-dlp `--cookies`.
//!
//! * Desktop: a secondary `WebviewWindow` (no IPC capability) is opened on the
//!   site; closing it, or pressing "Save cookies" in the app, reads the
//!   cookies via `Webview::cookies_for_url`.
//! * Android: the `CookieBrowserPlugin` Kotlin overlay shows a full-screen
//!   WebView and returns `CookieManager` cookie headers when the user taps
//!   "Save cookies". Other apps' cookie jars (e.g. Chrome's) are not readable
//!   on Android, which is why the login happens in-app.

use crate::error::{AppError, AppResult};
use serde::Serialize;
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{AppHandle, Runtime};

/// Lifetime given to session / undated cookies so yt-dlp and the vault keep them.
const SESSION_COOKIE_TTL_SECS: i64 = 30 * 24 * 3600;
/// Fallback lifetime for header-only cookies (Android `CookieManager` hides expiry).
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
const HEADER_COOKIE_TTL_SECS: i64 = 365 * 24 * 3600;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedCookie {
    pub domain: String,
    pub include_subdomains: bool,
    pub path: String,
    pub secure: bool,
    pub expires: i64,
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CookieLoginResult {
    pub site_id: String,
    /// Netscape cookie file contents, ready for `save_site_cookies`.
    pub netscape: String,
    pub count: usize,
}

/// A video page as rendered by the in-app browser after any bot check
/// (Android `CookieBrowserPlugin.resolvePage`).
#[derive(Debug, Clone, serde::Deserialize)]
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub struct PageCapture {
    pub html: String,
    /// Final page URL (after redirects).
    pub url: String,
    #[serde(rename = "userAgent", default)]
    pub user_agent: String,
    /// `Cookie:` header for the page URL.
    #[serde(default)]
    pub cookies: String,
}

/// Where a link led inside the in-app browser session.
#[derive(Debug, Clone, serde::Deserialize)]
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub struct FollowResult {
    pub url: String,
    #[serde(default)]
    pub cookies: String,
}

/// Open `url` in the in-app browser until no bot check is showing and
/// return the rendered page. The browser stays open for [`follow_in_page`]
/// until [`close_page`].
#[cfg(target_os = "android")]
pub use platform::{close_page, follow as follow_in_page, open_page};

/// Desktop: pass a bot check in a browser window and return its cookies
/// (see `sites::clearance`).
#[cfg(desktop)]
pub use platform::clear_bot_check;

/// Where the login browser starts and which URLs' cookies to collect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginTarget {
    pub start_url: String,
    pub cookie_urls: Vec<String>,
}

pub fn login_target(site_id: &str, base_url: &str) -> LoginTarget {
    let base = format!("{}/", base_url.trim_end_matches('/'));
    let (start, extra): (&str, &[&str]) = match site_id {
        "reddit" => (
            "https://www.reddit.com/login/",
            &[
                "https://www.reddit.com/",
                "https://old.reddit.com/",
                "https://reddit.com/",
            ],
        ),
        "twitter" => (
            "https://x.com/i/flow/login",
            &["https://x.com/", "https://twitter.com/"],
        ),
        "instagram" => ("https://www.instagram.com/accounts/login/", &[]),
        "tiktok" => ("https://www.tiktok.com/login", &[]),
        "youtube" => (
            "https://accounts.google.com/ServiceLogin?service=youtube&continue=https%3A%2F%2Fwww.youtube.com%2F",
            &["https://www.youtube.com/"],
        ),
        "redgifs" => ("https://www.redgifs.com/login", &[]),
        _ => ("", &[]),
    };
    let mut cookie_urls = vec![base.clone()];
    for u in extra {
        if !cookie_urls.iter().any(|c| c == u) {
            cookie_urls.push((*u).to_string());
        }
    }
    LoginTarget {
        start_url: if start.is_empty() {
            base
        } else {
            start.to_string()
        },
        cookie_urls,
    }
}

/// Netscape cookie file text. HttpOnly cookies are written as ordinary lines
/// (the `#HttpOnly_` prefix would be dropped by simple parsers).
pub fn to_netscape(cookies: &[CapturedCookie]) -> String {
    let mut out =
        String::from("# Netscape HTTP Cookie File\n# Captured by ArcHive login browser\n\n");
    for c in cookies {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            c.domain,
            if c.include_subdomains {
                "TRUE"
            } else {
                "FALSE"
            },
            c.path,
            if c.secure { "TRUE" } else { "FALSE" },
            c.expires,
            c.name,
            c.value
        ));
    }
    out
}

/// Drop duplicates (same domain/path/name) keeping the first occurrence, and
/// cookies whose name/value would corrupt the tab-separated format.
pub fn dedupe(cookies: Vec<CapturedCookie>) -> Vec<CapturedCookie> {
    let mut seen = std::collections::HashSet::new();
    cookies
        .into_iter()
        .filter(|c| {
            !c.name.is_empty()
                && !c.name.contains(['\t', '\n', '\r', ';', '='])
                && !c.value.contains(['\t', '\n', '\r'])
        })
        .filter(|c| seen.insert((c.domain.clone(), c.path.clone(), c.name.clone())))
        .collect()
}

/// Cookies from a `Cookie:`-style header (`a=1; b=2`) captured for `url`.
/// The header carries no domain/expiry, so cookies are scoped to the site's
/// registrable-ish host (`www.` stripped, subdomains included).
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub fn cookies_from_header(url: &str, header: &str, now: i64) -> Vec<CapturedCookie> {
    let Ok(parsed) = url::Url::parse(url) else {
        return Vec::new();
    };
    let Some(host) = parsed.host_str() else {
        return Vec::new();
    };
    let domain = format!(".{}", host.trim_start_matches("www."));
    let secure = parsed.scheme() == "https";
    header
        .split(';')
        .filter_map(|pair| {
            let (name, value) = pair.split_once('=')?;
            let name = name.trim();
            (!name.is_empty()).then(|| CapturedCookie {
                domain: domain.clone(),
                include_subdomains: true,
                path: "/".to_string(),
                secure,
                expires: now + HEADER_COOKIE_TTL_SECS,
                name: name.to_string(),
                value: value.trim().to_string(),
            })
        })
        .collect()
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Open the login browser for `site_id`, wait for the user to finish, and
/// return the captured cookies as a Netscape file.
pub async fn capture(
    app: &AppHandle,
    site_id: &str,
    display_name: &str,
    target: &LoginTarget,
) -> AppResult<CookieLoginResult> {
    let title = format!("Log in to {display_name}, then save cookies");
    let cookies = dedupe(platform::capture(app, &title, target).await?);
    if cookies.is_empty() {
        return Err(AppError::Site(format!(
            "No cookies were captured for {display_name}. Make sure the page finished loading and you are logged in."
        )));
    }
    Ok(CookieLoginResult {
        site_id: site_id.to_string(),
        count: cookies.len(),
        netscape: to_netscape(&cookies),
    })
}

/// Desktop: finish (save=true) or cancel the open login window.
pub fn finish(app: &AppHandle, save: bool) -> AppResult<()> {
    platform::finish(app, save)
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("cookie-browser")
        .setup(|app, api| {
            platform::setup(app, api);
            Ok(())
        })
        .build()
}

#[cfg(desktop)]
mod platform {
    use super::*;
    use std::sync::Mutex;
    use tauri::{Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
    use tokio::sync::mpsc;

    const LABEL: &str = "cookie-login";

    #[derive(Default)]
    pub struct DesktopLoginState {
        signal: Mutex<Option<mpsc::UnboundedSender<bool>>>,
    }

    pub fn setup<R: Runtime>(app: &AppHandle<R>, _api: tauri::plugin::PluginApi<R, ()>) {
        app.manage(DesktopLoginState::default());
    }

    fn send(app: &AppHandle, save: bool) -> bool {
        app.try_state::<DesktopLoginState>()
            .and_then(|s| s.signal.lock().ok().and_then(|g| g.clone()))
            .is_some_and(|tx| tx.send(save).is_ok())
    }

    pub fn finish(app: &AppHandle, save: bool) -> AppResult<()> {
        if send(app, save) {
            Ok(())
        } else {
            Err(AppError::Other("No login window is open.".into()))
        }
    }

    /// Every cookie the window holds for `urls`, HttpOnly ones included.
    fn window_cookies(window: &tauri::WebviewWindow, urls: &[String]) -> Vec<CapturedCookie> {
        let mut out = Vec::new();
        let now = now_unix();
        for u in urls {
            let Ok(parsed) = url::Url::parse(u) else {
                continue;
            };
            let host = parsed.host_str().unwrap_or_default().to_string();
            // Must run off the main thread (Windows deadlocks otherwise);
            // this is an async command task, so that holds.
            let cookies = match window.cookies_for_url(parsed) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("[cookie-login] cookies_for_url({u}) failed: {e}");
                    continue;
                }
            };
            for c in cookies {
                let (domain, include_subdomains) = match c.domain() {
                    Some(d) if !d.is_empty() => {
                        let d = if d.starts_with('.') || d == host {
                            d.to_string()
                        } else {
                            format!(".{d}")
                        };
                        let sub = d.starts_with('.');
                        (d, sub)
                    }
                    _ => (host.clone(), false),
                };
                let expires = c
                    .expires_datetime()
                    .map(|t| t.unix_timestamp())
                    .filter(|t| *t > now)
                    .unwrap_or(now + SESSION_COOKIE_TTL_SECS);
                out.push(CapturedCookie {
                    domain,
                    include_subdomains,
                    path: c
                        .path()
                        .filter(|p| !p.is_empty())
                        .unwrap_or("/")
                        .to_string(),
                    secure: c.secure().unwrap_or(false),
                    expires,
                    name: c.name().to_string(),
                    value: c.value().to_string(),
                });
            }
        }
        out
    }

    /// Show `url` in a small window (sending `user_agent`) until no bot
    /// check is showing, then return its cookies. Closes by itself; closing
    /// it by hand cancels.
    pub async fn clear_bot_check(
        app: &AppHandle,
        url: &str,
        host: &str,
        user_agent: &str,
    ) -> AppResult<Vec<CapturedCookie>> {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        // A fresh label per window: destroying the previous one is async, so
        // reusing a label can fail with "webview already exists".
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let label = format!("bot-check-{}", SEQ.fetch_add(1, Ordering::Relaxed));
        let start: url::Url = url
            .parse()
            .map_err(|e| AppError::InvalidInput(format!("bad URL: {e}")))?;
        let title = Arc::new(Mutex::new(String::new()));
        let loaded = Arc::new(AtomicBool::new(false));
        let closed = Arc::new(AtomicBool::new(false));
        let (t, l) = (title.clone(), loaded.clone());
        let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(start))
            .title(format!("Verifying {host} — closes by itself"))
            .inner_size(720.0, 640.0)
            .user_agent(user_agent)
            .focused(true)
            .on_document_title_changed(move |_, new_title| {
                if let Ok(mut g) = t.lock() {
                    *g = new_title;
                }
            })
            .on_page_load(move |_, payload| {
                l.store(
                    matches!(payload.event(), tauri::webview::PageLoadEvent::Finished),
                    Ordering::SeqCst,
                );
            })
            .build()
            .map_err(|e| AppError::Other(format!("could not open the browser window: {e}")))?;
        let c = closed.clone();
        window.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                c.store(true, Ordering::SeqCst);
            }
        });

        let started = std::time::Instant::now();
        let mut passed = false;
        // ARCHHIVE_BOT_CHECK_TIMEOUT_SECS: how long to wait for the check
        // to pass (default 150 s).
        let limit = std::env::var("ARCHHIVE_BOT_CHECK_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(150);
        while started.elapsed() < std::time::Duration::from_secs(limit) {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if closed.load(Ordering::SeqCst) {
                break;
            }
            let current = title.lock().map(|g| g.clone()).unwrap_or_default();
            if loaded.load(Ordering::SeqCst)
                && !current.trim().is_empty()
                && !crate::sites::challenge::is_challenge_title(&current)
            {
                // Let the cleared page settle its cookies.
                tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
                passed = true;
                break;
            }
        }
        let cookies = if passed {
            let page = window
                .url()
                .map(|u| u.to_string())
                .unwrap_or_else(|_| url.to_string());
            let mut urls = vec![page];
            if let Ok(u) = url::Url::parse(url) {
                urls.push(format!(
                    "{}://{}/",
                    u.scheme(),
                    u.host_str().unwrap_or_default()
                ));
            }
            window_cookies(&window, &urls)
        } else {
            Vec::new()
        };
        let _ = window.destroy();
        if !passed {
            return Err(AppError::Other(if closed.load(Ordering::SeqCst) {
                "Verification window closed.".into()
            } else {
                format!("{host} kept showing its bot check.")
            }));
        }
        Ok(dedupe(cookies))
    }

    pub async fn capture(
        app: &AppHandle,
        title: &str,
        target: &LoginTarget,
    ) -> AppResult<Vec<CapturedCookie>> {
        if let Some(existing) = app.get_webview_window(LABEL) {
            send(app, false);
            let _ = existing.destroy();
        }
        let start: url::Url = target
            .start_url
            .parse()
            .map_err(|e| AppError::InvalidInput(format!("bad login url: {e}")))?;

        let (tx, mut rx) = mpsc::unbounded_channel::<bool>();
        let state = app
            .try_state::<DesktopLoginState>()
            .ok_or_else(|| AppError::Other("cookie-browser plugin not initialized".into()))?;
        if let Ok(mut slot) = state.signal.lock() {
            *slot = Some(tx.clone());
        }

        let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::External(start))
            .title(title)
            .inner_size(1024.0, 780.0)
            .focused(true)
            .build()
            .map_err(|e| AppError::Other(format!("could not open login window: {e}")))?;
        // Closing the window means "done": keep it alive until cookies are read.
        window.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = tx.send(true);
            }
        });

        let save = rx.recv().await.unwrap_or(false);
        if let Ok(mut slot) = state.signal.lock() {
            *slot = None;
        }

        let out = if save {
            window_cookies(&window, &target.cookie_urls)
        } else {
            Vec::new()
        };
        let _ = window.destroy();
        if !save {
            return Err(AppError::Other("Login cancelled.".into()));
        }
        Ok(out)
    }
}

#[cfg(target_os = "android")]
mod platform {
    use super::*;
    use serde::Deserialize;
    use tauri::plugin::PluginHandle;
    use tauri::Manager;

    pub struct AndroidLoginState<R: Runtime> {
        handle: Option<PluginHandle<R>>,
    }

    #[derive(Deserialize)]
    struct Entry {
        url: String,
        header: String,
    }

    #[derive(Deserialize)]
    struct Response {
        #[serde(default)]
        cookies: Vec<Entry>,
    }

    pub fn setup<R: Runtime>(app: &AppHandle<R>, api: tauri::plugin::PluginApi<R, ()>) {
        let handle = match api.register_android_plugin("com.archhive.app", "CookieBrowserPlugin") {
            Ok(h) => Some(h),
            Err(e) => {
                tracing::warn!("CookieBrowserPlugin registration failed: {e}");
                None
            }
        };
        app.manage(AndroidLoginState { handle });
    }

    pub fn finish(_app: &AppHandle, _save: bool) -> AppResult<()> {
        // The native dialog has its own Save / Cancel buttons.
        Ok(())
    }

    pub async fn capture(
        app: &AppHandle,
        title: &str,
        target: &LoginTarget,
    ) -> AppResult<Vec<CapturedCookie>> {
        let handle = app
            .try_state::<AndroidLoginState<tauri::Wry>>()
            .and_then(|s| s.handle.clone())
            .ok_or_else(|| {
                AppError::Other(
                    "Login browser unavailable (CookieBrowserPlugin not registered). Re-run android:patches and rebuild the APK."
                        .into(),
                )
            })?;
        let payload = serde_json::json!({
            "url": target.start_url,
            "cookieUrls": target.cookie_urls,
            "title": title,
        });
        // Blocks until the user taps Save / Cancel: keep it off the async workers.
        let response: Response = tokio::task::spawn_blocking(move || {
            handle.run_mobile_plugin::<Response>("open", payload)
        })
        .await
        .map_err(|e| AppError::Other(format!("login browser task failed: {e}")))?
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("cancelled") {
                AppError::Other("Login cancelled.".into())
            } else {
                AppError::Other(format!("Login browser failed: {msg}"))
            }
        })?;
        let now = now_unix();
        Ok(response
            .cookies
            .iter()
            .flat_map(|e| cookies_from_header(&e.url, &e.header, now))
            .collect())
    }

    fn plugin(app: &AppHandle) -> AppResult<PluginHandle<tauri::Wry>> {
        app.try_state::<AndroidLoginState<tauri::Wry>>()
            .and_then(|s| s.handle.clone())
            .ok_or_else(|| AppError::Other("In-app browser unavailable on this build.".into()))
    }

    async fn call<T: serde::de::DeserializeOwned + Send + 'static>(
        app: &AppHandle,
        command: &'static str,
        payload: serde_json::Value,
    ) -> AppResult<T> {
        let handle = plugin(app)?;
        tokio::task::spawn_blocking(move || handle.run_mobile_plugin::<T>(command, payload))
            .await
            .map_err(|e| AppError::Other(format!("in-app browser task failed: {e}")))?
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("cancelled") {
                    AppError::Other("Cancelled in the in-app browser.".into())
                } else {
                    AppError::Other(format!("In-app browser: {msg}"))
                }
            })
    }

    pub async fn open_page(app: &AppHandle, url: &str, title: &str) -> AppResult<PageCapture> {
        call(
            app,
            "resolvePage",
            serde_json::json!({ "url": url, "title": title }),
        )
        .await
    }

    pub async fn follow(app: &AppHandle, url: &str, referer: &str) -> AppResult<FollowResult> {
        call(
            app,
            "followRedirect",
            serde_json::json!({ "url": url, "referer": referer }),
        )
        .await
    }

    pub async fn close_page(app: &AppHandle) {
        let _ = call::<serde_json::Value>(app, "closeResolver", serde_json::json!({})).await;
    }

    /// In-app browser for watching a page (ad blocking, per-site clearing,
    /// share); returns when the user closes it.
    pub async fn browse(app: &AppHandle, url: &str, title: &str) -> AppResult<()> {
        call::<serde_json::Value>(
            app,
            "browse",
            serde_json::json!({ "url": url, "title": title, "adblock": true }),
        )
        .await
        .map(|_| ())
    }

    /// Android share sheet for a URL / text or a local file.
    pub async fn share(app: &AppHandle, text: &str, path: &str, mime: &str) -> AppResult<()> {
        call::<serde_json::Value>(
            app,
            "share",
            serde_json::json!({ "text": text, "path": path, "mime": mime }),
        )
        .await
        .map(|_| ())
    }
}

#[cfg(all(mobile, not(target_os = "android")))]
mod platform {
    use super::*;

    pub fn setup<R: Runtime>(_app: &AppHandle<R>, _api: tauri::plugin::PluginApi<R, ()>) {}

    pub fn finish(_app: &AppHandle, _save: bool) -> AppResult<()> {
        Ok(())
    }

    pub async fn capture(
        _app: &AppHandle,
        _title: &str,
        _target: &LoginTarget,
    ) -> AppResult<Vec<CapturedCookie>> {
        Err(AppError::Other(
            "The login browser is not available on this platform yet.".into(),
        ))
    }
}

/// In-app browser (Android) for a page the app can't play itself.
pub async fn browse(app: &AppHandle, url: &str, title: &str) -> AppResult<()> {
    #[cfg(target_os = "android")]
    {
        platform::browse(app, url, title).await
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (app, url, title);
        Err(AppError::Other(
            "The in-app browser is only available on Android.".into(),
        ))
    }
}

/// Android share sheet: `text` (a URL) or `path` (a library file, `mime`).
pub async fn share(app: &AppHandle, text: &str, path: &str, mime: &str) -> AppResult<()> {
    #[cfg(target_os = "android")]
    {
        platform::share(app, text, path, mime).await
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (app, text, path, mime);
        Err(AppError::Other(
            "Sharing is only available on Android.".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_cookies_scope_to_site_domain() {
        let c = cookies_from_header(
            "https://www.reddit.com/",
            "reddit_session=abc; token_v2=x=y",
            100,
        );
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].domain, ".reddit.com");
        assert!(c[0].include_subdomains && c[0].secure);
        assert_eq!(c[0].expires, 100 + HEADER_COOKIE_TTL_SECS);
        assert_eq!(c[1].name, "token_v2");
        assert_eq!(c[1].value, "x=y");
    }

    #[test]
    fn netscape_output_round_trips_through_vault_format() {
        let cookies = dedupe(vec![
            CapturedCookie {
                domain: ".example.com".into(),
                include_subdomains: true,
                path: "/".into(),
                secure: true,
                expires: 2_000_000_000,
                name: "sid".into(),
                value: "v1".into(),
            },
            CapturedCookie {
                domain: ".example.com".into(),
                include_subdomains: true,
                path: "/".into(),
                secure: true,
                expires: 2_000_000_000,
                name: "sid".into(),
                value: "dupe".into(),
            },
            CapturedCookie {
                domain: "example.com".into(),
                include_subdomains: false,
                path: "/".into(),
                secure: false,
                expires: 1,
                name: "bad\tname".into(),
                value: "x".into(),
            },
        ]);
        assert_eq!(cookies.len(), 1);
        let text = to_netscape(&cookies);
        assert!(text.starts_with("# Netscape HTTP Cookie File"));
        assert!(text.contains(".example.com\tTRUE\t/\tTRUE\t2000000000\tsid\tv1\n"));
    }

    #[test]
    fn login_targets() {
        let r = login_target("reddit", "https://old.reddit.com");
        assert_eq!(r.start_url, "https://www.reddit.com/login/");
        assert!(r
            .cookie_urls
            .contains(&"https://old.reddit.com/".to_string()));
        let x = login_target("xvideos", "https://www.xvideos.com");
        assert_eq!(x.start_url, "https://www.xvideos.com/");
        assert_eq!(x.cookie_urls, vec!["https://www.xvideos.com/".to_string()]);
    }
}
