//! Cookie-harvest bridge for Android (in-app WebView login capture).
//!
//! The Kotlin `CookieHarvestPlugin` (installed by `scripts/patch-android-cookie.ps1`)
//! owns a plain `CookieLoginActivity` that opens a site login page in an in-app
//! WebView. Capture is **manual** — the window stays open until the user taps
//! "Save cookies" (Cancel/Back/timeout save nothing), so a half-finished multi-step
//! or OAuth login can never overwrite a good session. The finished state sits in a
//! static slot and the frontend pulls it with the `poll_cookie_harvest` command
//! (take + clear) — same pure pull model as the share target, so results survive the
//! user switching back to ArcHive.
//!
//! Android only: on desktop `open_login` returns a clear error pointing at the
//! existing clipboard / DevTools import flows.

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use tauri::plugin::{Builder, PluginHandle, TauriPlugin};
use tauri::{AppHandle, Manager, Runtime};

const PLUGIN_UNAVAILABLE: &str = "Cookie harvest plugin unavailable (CookieHarvestPlugin not registered). Re-run bun run android:patches and rebuild the APK.";

/// Site id → login page opened in the harvest window.
///
/// Covers every adapter that `requires_cookies` plus the sites the frontend exposes
/// in the Cookies tab. Unknown sites fall back to the adapter base URL; callers
/// should prefer [`login_url_for_site`].
///
/// Entry-point notes:
/// - `thothub.tv` is a **parked ad domain** now (it redirects to a casino landing
///   page); the real site is `thethothub.com` (mirrors: `thothub.to`, `thothub.org`).
/// - ThotHub's login is a fancybox *popup fragment*: visiting `/login/` directly
///   renders the bare form with no stylesheet or site chrome, which looks like a
///   dodgy 2001-era page. Open the homepage instead and use its own login modal.
pub const SITE_LOGINS: &[(&str, &str)] = &[
    ("chaturbate", "https://chaturbate.com/auth/login/"),
    ("stripchat", "https://stripchat.com/"),
    ("thothub", "https://thethothub.com/"),
    ("eporner", "https://www.eporner.com/login/"),
    ("pornhub", "https://www.pornhub.com/login"),
    ("youporn", "https://www.youporn.com/login/"),
    ("xnxx", "https://www.xnxx.com/"),
    ("xhamster", "https://xhamster.com/login"),
    ("xvideos", "https://www.xvideos.com/account"),
    ("reddit", "https://www.reddit.com/login"),
    ("redgifs", "https://www.redgifs.com/login"),
    ("thisvid", "https://thisvid.com/login/"),
    ("youtube", "https://accounts.google.com/ServiceLogin"),
    ("tiktok", "https://www.tiktok.com/login"),
    ("instagram", "https://www.instagram.com/accounts/login/"),
    ("twitter", "https://x.com/i/flow/login"),
];

/// Login URL for a known site id.
pub fn login_url_for_site(site_id: &str) -> Option<&'static str> {
    SITE_LOGINS
        .iter()
        .find(|(id, _)| *id == site_id)
        .map(|(_, url)| *url)
}

pub struct CookieHarvestPluginState<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: Option<PluginHandle<R>>,
    #[cfg(not(target_os = "android"))]
    _marker: std::marker::PhantomData<R>,
}

/// A finished harvest, handed to the frontend for vault import.
#[derive(Debug, Clone, Serialize)]
pub struct CookieHarvest {
    pub site_id: String,
    /// "k=v; k2=v2" cookie header (non-HttpOnly cookies of the session domain).
    pub cookie_header: String,
    /// Host of the login page; used as the Netscape cookie domain.
    pub domain: String,
}

impl CookieHarvest {
    /// Whether this harvest carries anything worth importing.
    pub fn is_usable(&self) -> bool {
        !self.cookie_header.trim().is_empty()
    }
}

/// State of the in-app login window.
///
/// `Harvest`/`Timeout` are only produced by the Android plugin bridge, so they read
/// as dead code on desktop builds.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HarvestState {
    /// The window is still open — keep polling.
    Pending,
    /// The user tapped "Save cookies"; [`HarvestPoll::harvest`] is populated.
    Harvest,
    /// The user cancelled / pressed Back; nothing is saved.
    Cancelled,
    /// The 10 minute limit elapsed; nothing is saved.
    Timeout,
}

/// Result of one poll of the in-app login window.
#[derive(Debug, Clone, Serialize)]
pub struct HarvestPoll {
    pub state: HarvestState,
    /// Only set when `state == HarvestState::Harvest`.
    pub harvest: Option<CookieHarvest>,
}

impl HarvestPoll {
    fn pending() -> Self {
        Self {
            state: HarvestState::Pending,
            harvest: None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HarvestResponse {
    state: Option<String>,
    site_id: Option<String>,
    cookie_header: Option<String>,
    domain: Option<String>,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("cookieHarvest")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                match api.register_android_plugin("com.archhive.app", "CookieHarvestPlugin") {
                    Ok(handle) => {
                        app.manage(CookieHarvestPluginState {
                            handle: Some(handle),
                        });
                    }
                    Err(e) => {
                        eprintln!("[cookieHarvest] Failed to register CookieHarvestPlugin: {e}");
                        tracing::warn!("Cookie harvest registration failed: {e}");
                        app.manage(CookieHarvestPluginState::<R> { handle: None });
                    }
                }
            }
            #[cfg(not(target_os = "android"))]
            {
                let _ = (app, api);
            }
            Ok(())
        })
        .build()
}

/// Open the in-app login window for `site_id` at `url`.
pub fn open_login(app: &AppHandle, site_id: &str, url: &str) -> AppResult<()> {
    #[cfg(target_os = "android")]
    {
        let state = app
            .try_state::<CookieHarvestPluginState<tauri::Wry>>()
            .ok_or_else(|| AppError::Download(PLUGIN_UNAVAILABLE.into()))?;
        let handle = state
            .handle
            .as_ref()
            .ok_or_else(|| AppError::Download(PLUGIN_UNAVAILABLE.into()))?;
        handle
            .run_mobile_plugin::<serde_json::Value>(
                "openCookieLogin",
                serde_json::json!({ "siteId": site_id, "url": url }),
            )
            .map_err(|e| AppError::Download(format!("cookie harvest open failed: {e}")))?;
        Ok(())
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (app, site_id, url);
        Err(AppError::Download(
            "Cookie harvest needs the Android app. Use the DevTools table / Cookie-Editor JSON import here."
                .into(),
        ))
    }
}

/// Take + clear the pending harvest state. `Pending` when the window is still open.
pub fn poll(app: &AppHandle) -> AppResult<HarvestPoll> {
    #[cfg(target_os = "android")]
    {
        let state = app
            .try_state::<CookieHarvestPluginState<tauri::Wry>>()
            .ok_or_else(|| AppError::Download(PLUGIN_UNAVAILABLE.into()))?;
        let handle = state
            .handle
            .as_ref()
            .ok_or_else(|| AppError::Download(PLUGIN_UNAVAILABLE.into()))?;
        let response: HarvestResponse = handle
            .run_mobile_plugin("takeHarvest", serde_json::json!({}))
            .map_err(|e| AppError::Download(format!("cookie harvest poll failed: {e}")))?;
        Ok(match response.state.as_deref() {
            Some("harvest") => {
                let harvest = CookieHarvest {
                    site_id: response.site_id.unwrap_or_default(),
                    cookie_header: response.cookie_header.unwrap_or_default(),
                    domain: response.domain.unwrap_or_default(),
                };
                if harvest.is_usable() {
                    HarvestPoll {
                        state: HarvestState::Harvest,
                        harvest: Some(harvest),
                    }
                } else {
                    // "Save" with no cookies: nothing to import, but the window is
                    // closed, so report it as cancelled rather than polling forever.
                    HarvestPoll {
                        state: HarvestState::Cancelled,
                        harvest: None,
                    }
                }
            }
            Some("cancelled") => HarvestPoll {
                state: HarvestState::Cancelled,
                harvest: None,
            },
            Some("timeout") => HarvestPoll {
                state: HarvestState::Timeout,
                harvest: None,
            },
            _ => HarvestPoll::pending(),
        })
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Ok(HarvestPoll::pending())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_sites_resolve_to_login_urls() {
        // Regression: thothub.tv is a parked ad domain now, and ThotHub's /login/ path is
        // a bare popup fragment with no stylesheet (looks like a scam page) — the capture
        // window must open the real site instead.
        let thothub = login_url_for_site("thothub").unwrap();
        assert_eq!(thothub, "https://thethothub.com/");
        assert!(!thothub.contains("thothub.tv"), "{thothub}");
        assert!(!thothub.ends_with("/login/"), "{thothub}");
        assert!(login_url_for_site("chaturbate")
            .unwrap()
            .contains("chaturbate"));
        assert!(login_url_for_site("redgifs").unwrap().contains("redgifs"));
    }

    /// The Cookies tab offers capture for these; they must all have a login page.
    #[test]
    fn extra_capture_sites_are_covered() {
        for id in ["twitter", "eporner", "xnxx", "xhamster", "youporn"] {
            assert!(
                login_url_for_site(id).is_some(),
                "missing login url for {id}"
            );
        }
    }

    #[test]
    fn unknown_site_has_no_login_url() {
        assert!(login_url_for_site("does-not-exist").is_none());
    }

    #[test]
    fn every_login_entry_is_unique_and_https() {
        let mut seen = std::collections::HashSet::new();
        for (id, url) in SITE_LOGINS {
            assert!(seen.insert(*id), "duplicate site id: {id}");
            assert!(url.starts_with("https://"), "{id} is not https: {url}");
        }
    }

    #[test]
    fn blank_harvest_is_not_usable() {
        let harvest = CookieHarvest {
            site_id: "thothub".into(),
            cookie_header: "   ".into(),
            domain: "thethothub.com".into(),
        };
        assert!(!harvest.is_usable());
    }

    #[test]
    fn poll_states_serialize_snake_case() {
        let json = serde_json::to_string(&HarvestPoll {
            state: HarvestState::Cancelled,
            harvest: None,
        })
        .unwrap();
        assert!(json.contains("\"cancelled\""), "{json}");
        let json = serde_json::to_string(&HarvestPoll::pending()).unwrap();
        assert!(json.contains("\"pending\""), "{json}");
    }
}
