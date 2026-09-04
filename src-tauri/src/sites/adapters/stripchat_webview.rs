//! Hidden Tauri WebView bridge for Stripchat listings.
//!
//! Stripchat's room list is hydrated client-side by a SPA bundle.
//! Server-rendered HTML only contains placeholder markup, so static `reqwest`
//! scraping returns nothing useful. This module spawns a hidden 1x1
//! `WebviewWindow` pointing at the listing URL, injects Stripchat cookies
//! from the vault, waits for the SPA to render real room cards via
//! `MutationObserver`, then ships the extracted room data back to Rust
//! through a custom-scheme navigation.

use crate::error::AppResult;
use crate::vault::CookieVault;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::webview::PageLoadEvent;
use tauri::{webview::WebviewWindowBuilder, AppHandle, Url, WebviewUrl};
use tokio::sync::oneshot;
use uuid::Uuid;

/// Custom URL scheme used as a JS -> Rust return channel.
const _BRIDGE_SCHEME: &str = "strip-bridge";

/// How long the bridge waits for the SPA to render before giving up.
const _BRIDGE_TIMEOUT: Duration = Duration::from_secs(12);

/// Raw room data extracted by the injected JS.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct _RawRoom {
    pub username: String,
    pub title: String,
    #[serde(default)]
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub viewers: Option<u32>,
    #[serde(default)]
    pub age: Option<u32>,
    #[serde(default)]
    pub gender: Option<String>,
}

/// Spawn a hidden webview pointed at `url`, inject cookies from the vault,
/// wait for the SPA to hydrate and emit room data.
pub async fn _fetch_listing(
    app: &AppHandle,
    vault: &CookieVault,
    url: &str,
) -> AppResult<Vec<_RawRoom>> {
    let label = format!("strip-bridge-{}", Uuid::new_v4().simple());

    let cookie_header = vault
        .cookie_header("stripchat")
        .unwrap_or(None)
        .unwrap_or_default();

    let (tx, rx) = oneshot::channel::<Option<Vec<_RawRoom>>>();
    let tx_guard = Arc::new(Mutex::new(Some(tx)));

    let cookie_init = if cookie_header.is_empty() {
        String::new()
    } else {
        let assigns: Vec<String> = cookie_header
            .split("; ")
            .filter(|s| !s.is_empty())
            .map(|kv| {
                let escaped = kv.replace('\\', "\\\\").replace('\'', "\\'");
                format!("document.cookie = '{escaped}';")
            })
            .collect();
        assigns.join("\n")
    };

    let init_script = format!(
        "window.__STRIP_BRIDGE_LABEL__ = '{label}';\n{cookie_init}\nwindow.__STRIP_BRIDGE_DONE__ = false;",
        label = label.replace('\'', "\\'"),
    );

    let parsed_url = match Url::parse(url) {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!("[strip-bridge] failed to parse listing URL {url}: {e}");
            return Ok(vec![]);
        }
    };

    let builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(parsed_url))
        .inner_size(1.0, 1.0)
        .visible(false)
        .initialization_script(init_script)
        .on_page_load(move |wv, payload| {
            if matches!(payload.event(), PageLoadEvent::Finished) {
                if let Err(e) = wv.eval(_EXTRACT_JS) {
                    tracing::warn!("[strip-bridge] eval EXTRACT_JS failed: {e}");
                }
            }
        })
        .on_navigation({
            let tx_guard = tx_guard.clone();
            move |nav_url| {
                if nav_url.scheme() != _BRIDGE_SCHEME {
                    return true;
                }

                let report = |rooms: Option<Vec<_RawRoom>>| {
                    if let Some(tx) = tx_guard.lock().unwrap().take() {
                        let _ = tx.send(rooms);
                    }
                };

                let path = nav_url.path();
                if path != "done" {
                    report(None);
                    return false;
                }

                let raw_payload = nav_url
                    .query_pairs()
                    .find_map(|(k, v)| (k == "p").then_some(v))
                    .map(|s| s.to_string());

                let Some(b64) = raw_payload else {
                    report(None);
                    return false;
                };

                let json = match base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    b64.as_bytes(),
                ) {
                    Ok(bytes) => match String::from_utf8(bytes) {
                        Ok(s) => s,
                        Err(_) => {
                            report(None);
                            return false;
                        }
                    },
                    Err(_) => {
                        report(None);
                        return false;
                    }
                };

                match serde_json::from_str::<Vec<_RawRoom>>(&json) {
                    Ok(rooms) => report(Some(rooms)),
                    Err(e) => {
                        tracing::warn!("[strip-bridge] JSON decode failed: {e}");
                        report(None);
                    }
                }
                false
            }
        });

    let webview = match builder.build() {
        Ok(w) => w,
        Err(e) => {
            tracing::warn!("[strip-bridge] failed to build webview {label}: {e}");
            return Ok(vec![]);
        }
    };

    let result = match tokio::time::timeout(_BRIDGE_TIMEOUT, rx).await {
        Ok(Ok(Some(rooms))) => rooms,
        Ok(Ok(None)) => vec![],
        Ok(Err(_)) => vec![],
        Err(_) => {
            tracing::warn!("[strip-bridge] timed out after {_BRIDGE_TIMEOUT:?} for {label}");
            vec![]
        }
    };

    let _ = webview.close();

    Ok(result)
}

/// Extractor JS injected after `PageLoadEvent::Finished`.
///
/// Strategy:
/// 1. Look for room list containers (`.room-card`, `.model-card`, etc.).
/// 2. Use `MutationObserver` to detect when real hydrated cards appear.
/// 3. Walk all real cards and emit room data via `strip-bridge://done?p=<base64>`.
const _EXTRACT_JS: &str = r##"
(function () {
  function emit(rooms) {
    if (window.__STRIP_BRIDGE_DONE__) return;
    window.__STRIP_BRIDGE_DONE__ = true;
    try {
      var json = JSON.stringify(rooms);
      var b64 = btoa(unescape(encodeURIComponent(json)));
      window.location.href = "strip-bridge://done?p=" + encodeURIComponent(b64);
    } catch (e) {
      window.location.href = "strip-bridge://done?p=";
    }
  }

  function extractRoomFromEl(el) {
    var link = null;
    el.querySelectorAll("a[href]").forEach(function (a) {
      if (link) return;
      var href = a.getAttribute("href") || "";
      if (
        href.charAt(0) === "/" &&
        href.indexOf("/tags/") === -1 &&
        href.indexOf("/search") === -1 &&
        href.indexOf("/embed/") === -1 &&
        href.indexOf("/api/") === -1 &&
        href.indexOf("/?") === -1
      ) {
        var seg = href.replace(/^\/+/, "").replace(/\/+$/, "");
        seg = seg.split(/[?#]/)[0];
        if (seg && seg.indexOf("/") === -1) {
          link = { href: href, username: seg };
        }
      }
    });
    if (!link) return null;

    var title = "";
    var titleEl = el.querySelector(".title, .model-name, [data-testid='model-title']");
    if (titleEl) {
      title = (titleEl.textContent || "").trim();
    }
    if (!title) {
      var allText = (el.textContent || "").trim().split(/\s+/).join(" ");
      title = allText.length > 0 ? allText : link.username;
    }

    var thumb = null;
    el.querySelectorAll("img").forEach(function (img) {
      if (thumb) return;
      var src = img.getAttribute("src") || img.getAttribute("data-src") || "";
      if (src && !src.startsWith("data:")) {
        thumb = src.startsWith("http") ? src : (src.startsWith("//") ? "https:" + src : "https:" + src);
      }
    });

    var viewers = null;
    el.querySelectorAll("*").forEach(function (ele) {
      if (viewers !== null) return;
      var t = (ele.textContent || "").trim();
      var m = t.match(/(\d+)\s*(viewers|watching|people)/i);
      if (m) viewers = parseInt(m[1], 10);
    });

    var age = null;
    var ageMatch = title.match(/\b(1[8-9]|[2-9][0-9])\b/);
    if (ageMatch) {
      var a = parseInt(ageMatch[1], 10);
      if (a >= 18 && a <= 99) age = a;
    }

    var gender = null;
    var cls = (el.className || "") + " " + (el.parentElement ? el.parentElement.className || "" : "");
    if (/\bf\b|female|girl|woman/i.test(cls)) gender = "female";
    else if (/\bm\b|male|guy|man/i.test(cls)) gender = "male";
    else if (/\bc\b|couple/i.test(cls)) gender = "couple";
    else if (/\bt\b|trans/i.test(cls)) gender = "trans";

    return {
      username: link.username,
      title: title,
      thumbnail: thumb,
      viewers: viewers,
      age: age,
      gender: gender
    };
  }

  function collectRooms() {
    var items = [];
    var seen = {};
    document.querySelectorAll(
      ".room-card:not(.placeholder), " +
      ".model-card, " +
      ".models-list a[href^='/'], " +
      "[class*='roomCard'], " +
      "[class*='modelCard']"
    ).forEach(function (el) {
      var room = extractRoomFromEl(el);
      if (!room) return;
      if (!room.username || seen[room.username]) return;
      seen[room.username] = true;
      items.push(room);
      if (items.length >= 48) return;
    });
    return items;
  }

  function done() {
    emit(collectRooms());
  }

  // Try to find a reasonable container to observe.
  var root = document.querySelector(
    ".models-list, .room-list, [class*='roomList'], [class*='modelList'], #roomlist_root"
  );
  if (!root) {
    setTimeout(done, 4000);
    return;
  }

  if (root.querySelector(":scope > *:not(.placeholder)")) {
    setTimeout(done, 250);
    return;
  }

  var observer = new MutationObserver(function (mutations, obs) {
    if (root.querySelector(":scope > *:not(.placeholder)")) {
      obs.disconnect();
      setTimeout(done, 350);
    }
  });
  observer.observe(root, { childList: true, subtree: true });

  setTimeout(function () {
    observer.disconnect();
    done();
  }, 8000);
})();
"##;
