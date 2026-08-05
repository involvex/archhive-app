//! Hidden Tauri WebView bridge for Chaturbate listings.
//!
//! Chaturbate's room list is hydrated client-side by a `roomlist-prefetch` JS
//! bundle. Server-rendered HTML only contains placeholder `<li class="roomCard
//! placeholder">` cards, so static `reqwest` scraping returns nothing. This
//! module spawns a hidden 1x1 `WebviewWindow` pointing at the listing URL,
//! injects Chaturbate cookies from the vault, waits for the SPA to render
//! real (`:not(.placeholder)`) cards via `MutationObserver`, then ships the
//! extracted room data back to Rust through a custom-scheme navigation.
//!
//! The bridge runs entirely on the desktop app's main `tauri::AppHandle`; the
//! hidden webview inherits ArcHive's HTTP/cookie jar so existing session
//! cookies (age-gate, login) are honoured without server-side API workarounds.
//!
//! Cross-platform note: `on_navigation` + custom-scheme `location.href` is
//! supported on WebView2 (Windows), WKWebView (macOS), and WebKitGTK (Linux).
//! `WebviewWindowBuilder` noted on Windows: must be called from `async`
//! commands / threads, never from synchronous Tauri command handlers. The
//! entrance `fetch_listing` is `async fn` so that constraint is satisfied.

use crate::error::AppResult;
use crate::vault::CookieVault;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::webview::PageLoadEvent;
use tauri::{webview::WebviewWindowBuilder, AppHandle, Url, WebviewUrl};
use tokio::sync::oneshot;
use uuid::Uuid;

/// Custom URL scheme used as a JS -> Rust return channel. JS sets
/// `window.location.href = "cb-bridge://done?p=<base64-json>"` and Rust's
/// `on_navigation` callback decodes the payload and cancels the navigation.
const _BRIDGE_SCHEME: &str = "cb-bridge";

/// How long the bridge waits for the SPA to render before giving up.
/// 8 seconds is generous: in practice room cards appear within 1-2s on a
/// warm connection, 5-6s on a cold one. Past 8s we assume the page failed
/// to hydrate (e.g. age-gate cookie missing, region blockCDN, etc.).
const _BRIDGE_TIMEOUT: Duration = Duration::from_secs(8);

/// Raw room data extracted by the injected JS. Fields mirror what's visible
/// in the rendered card DOM; `null`/missing values come back as `None`.
///
/// Keep field names in `camelCase` because the JS bridge produces JSON with
/// camelCase keys (matches the JS-side style and `MediaItem` casing).
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
/// wait for the SPA to hydrate and emit room cards, and return them.
///
/// On ANY failure (webview build error, navigation timeout, JSON decode
/// error, age-gate cookie missing) this function returns `Ok(vec![])` and
/// logs a warning; never an `Err`. The listing page is best-effort —
/// surfacing the failure to the user as a hard error would be worse than
/// showing the (now-empty) grid with the existing "no rooms from this
/// listing" message.
pub async fn _fetch_listing(
    app: &AppHandle,
    vault: &CookieVault,
    url: &str,
) -> AppResult<Vec<_RawRoom>> {
    let label = format!("cb-bridge-{}", Uuid::new_v4().simple());

    let cookie_header = vault
        .cookie_header("chaturbate")
        .unwrap_or(None)
        .unwrap_or_default();

    // oneshot return channel: Some(rooms) on success, None on timeout/error.
    // Wrapped in Arc<Mutex<Option<..>>> so the Fn on_navigation closure can
    // take-and-send only on the first match (a bridge URL could surface more
    // than once during redirect chains, but we only need the first payload).
    let (tx, rx) = oneshot::channel::<Option<Vec<_RawRoom>>>();
    let tx_guard = Arc::new(Mutex::new(Some(tx)));

    // Build the initialization script. Cookies are injected by inserting
    // `<meta http-equiv="Set-Cookie">` won't work here (already past
    // document_start for SPA). Instead we directly assign `document.cookie`
    // for each cookie before the SPA bundle runs — that ensures the
    // age-verification cookie is present before the SPA's XHR fires.
    let cookie_init = if cookie_header.is_empty() {
        String::new()
    } else {
        // The vault returns "name1=val1; name2=val2; ...". Split, assign each.
        let assigns: Vec<String> = cookie_header
            .split("; ")
            .filter(|s| !s.is_empty())
            .map(|kv| {
                // Safe embed: escape backslash and single-quote in the value.
                let escaped = kv.replace('\\', "\\\\").replace('\'', "\\'");
                format!("document.cookie = '{escaped}';")
            })
            .collect();
        assigns.join("\n")
    };

    let init_script = format!(
        "window.__CB_BRIDGE_LABEL__ = '{label}';\n{cookie_init}\nwindow.__CB_BRIDGE_DONE__ = false;",
        label = label.replace('\'', "\\'"),
    );

    let parsed_url = match Url::parse(url) {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!("[cb-bridge] failed to parse listing URL {url}: {e}");
            return Ok(vec![]);
        }
    };

    // Build the window. Use oneshot for both the navigation callback and the
    // timeout path: if the navigation closure never fires (e.g. page render
    // froze), the `tokio::time::timeout` below returns the error and we
    // clean up the window manually.
    let builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(parsed_url))
        .inner_size(1.0, 1.0)
        .visible(false)
        .initialization_script(init_script)
        .on_page_load(move |wv, payload| {
            // On Finished, kick off the extractor JS. We don't touch the webview
            // for Started events because the SPA bundle hasn't run yet.
            if matches!(payload.event(), PageLoadEvent::Finished) {
                if let Err(e) = wv.eval(_EXTRACT_JS) {
                    tracing::warn!("[cb-bridge] eval EXTRACT_JS failed: {e}");
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
                        tracing::warn!("[cb-bridge] JSON decode failed: {e}");
                        report(None);
                    }
                }
                false
            }
        });

    let webview = match builder.build() {
        Ok(w) => w,
        Err(e) => {
            tracing::warn!("[cb-bridge] failed to build webview {label}: {e}");
            return Ok(vec![]);
        }
    };

    // Wait for the result with a timeout. If we time out, the SPA failed to
    // render in time — return an empty list and clean up the window.
    let result = match tokio::time::timeout(_BRIDGE_TIMEOUT, rx).await {
        Ok(Ok(Some(rooms))) => rooms,
        Ok(Ok(None)) => vec![],
        Ok(Err(_)) => vec![], // tx dropped without sending
        Err(_) => {
            tracing::warn!("[cb-bridge] timed out after {_BRIDGE_TIMEOUT:?} for {label}");
            vec![]
        }
    };

    // Drop the window either way. destroy() is more forceful than close()
    // (close can be intercepted by beforeunload handlers) — but for a hidden
    // throwaway window close() is sufficient and lets the WebView release
    // memory asynchronously.
    let _ = webview.close();

    Ok(result)
}

/// Extractor JS injected after `PageLoadEvent::Finished`.
///
/// Strategy:
/// 1. If the age-gate modal is present and we have cookies, the SPA should
///    auto-bypass it. If it's still visible after 1s, dispatch past it.
/// 2. Look for the room list root (`#roomlist_root` or `ul#room_list`).
/// 3. Use a `MutationObserver` to detect when at least one
///    `li:not(.placeholder)` (a real hydrated card) appears in the list.
/// 4. Once seen (or after an 8s timeout), walk all real cards and emit
///    `cb-bridge://done?p=<base64(json)>` via `location.href`.
///
/// The JS is wrapped in an IIFE so no window globals leak past completion.
const _EXTRACT_JS: &str = r##"
(function () {
  function emit(rooms) {
    if (window.__CB_BRIDGE_DONE__) return;
    window.__CB_BRIDGE_DONE__ = true;
    try {
      var json = JSON.stringify(rooms);
      var b64 = btoa(unescape(encodeURIComponent(json)));
      window.location.href = "cb-bridge://done?p=" + encodeURIComponent(b64);
    } catch (e) {
      window.location.href = "cb-bridge://done?p=";
    }
  }

  function extractRoomFromLi(li) {
    // Find the room link: first <a href="/<username>/"> whose href starts
    // with "/" and isn't a tag/search/embed path.
    var link = null;
    li.querySelectorAll("a[href]").forEach(function (a) {
      if (link) return;
      var href = a.getAttribute("href") || "";
      if (
        href.charAt(0) === "/" &&
        href.indexOf("/tags/") === -1 &&
        href.indexOf("/search/") === -1 &&
        href.indexOf("/embed/") === -1 &&
        href.indexOf("/?") === -1
      ) {
        // First segment only — strip leading/trailing slash and any
        // trailing query/fragment.
        var seg = href.replace(/^\/+/, "").replace(/\/+$/, "");
        seg = seg.split(/[?#]/)[0];
        if (seg && seg.indexOf("/") === -1) {
          link = { href: href, username: seg };
        }
      }
    });
    if (!link) return null;

    // Title — first meaningful text node in the card.
    var title = "";
    var titleEl = li.querySelector(".title, .roomCard__title, [data-testid='room-title']");
    if (titleEl) {
      title = (titleEl.textContent || "").trim();
    }
    if (!title) {
      // Fallback: longest trimmed line of text.
      var allText = (li.textContent || "").trim().split(/\s+/).join(" ");
      title = allText.length > 0 ? allText : link.username;
    }

    // Thumbnail — first img src/data-src that's not a data: URI.
    var thumb = null;
    li.querySelectorAll("img").forEach(function (img) {
      if (thumb) return;
      var src = img.getAttribute("src") || img.getAttribute("data-src") || "";
      if (src && !src.startsWith("data:")) {
        thumb = src.startsWith("http") ? src : (src.startsWith("//") ? "https:" + src : "https:" + src);
      }
    });

    // Viewers — number found in elements with class containing "viewers" or
    // text containing "viewers"/"watching".
    var viewers = null;
    li.querySelectorAll("*").forEach(function (el) {
      if (viewers !== null) return;
      var cls = el.className || "";
      var t = (el.textContent || "").trim();
      var m = t.match(/(\d+)\s*(viewers|watching|people)/i);
      if (m) viewers = parseInt(m[1], 10);
      else if (typeof cls === "string" && /viewer/i.test(cls)) {
        var n = t.match(/\d+/);
        if (n) viewers = parseInt(n[0], 10);
      }
    });

    // Age — two-digit number in title within 18-99 range.
    var age = null;
    var ageMatch = title.match(/\b(1[8-9]|[2-9][0-9])\b/);
    if (ageMatch) {
      var a = parseInt(ageMatch[1], 10);
      if (a >= 18 && a <= 99) age = a;
    }

    // Gender — guessed from class on the card (e.g. "f", "m", "c", "t").
    var gender = null;
    var cls = (li.className || "") + " " + (li.parentElement ? li.parentElement.className || "" : "");
    if (/\bf\b|female|girl/i.test(cls)) gender = "female";
    else if (/\bm\b|male|guy/i.test(cls)) gender = "male";
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
      "#roomlist_root li.roomCard:not(.placeholder), " +
      "#roomlist_root li:not(.placeholder), " +
      "ul#room_list li:not(.placeholder), " +
      ".roomCard:not(.placeholder)"
    ).forEach(function (li) {
      var room = extractRoomFromLi(li);
      if (!room) return;
      if (!room.username || seen[room.username]) return;
      seen[room.username] = true;
      items.push(room);
      if (items.length >= 48) return;
    });
    return items;
  }

  // Wait for the SPA to hydrate. MutationObserver catches both the case
  // where cards already exist when this script runs and the case where they
  // appear asynchronously. 8s timeout matches the Rust-side timeout.
  function done() {
    emit(collectRooms());
  }

  var root = document.querySelector("#roomlist_root, ul#room_list");
  if (!root) {
    setTimeout(done, 4000); // give the SPA time to even mount the root
    return;
  }

  // If cards are already there (page came back fast), collect immediately.
  if (root.querySelector("li:not(.placeholder), .roomCard:not(.placeholder)")) {
    setTimeout(done, 250);
    return;
  }

  var observer = new MutationObserver(function (mutations, obs) {
    if (root.querySelector("li:not(.placeholder), .roomCard:not(.placeholder)")) {
      obs.disconnect();
      // Small delay so all the per-room data gets written in before we
      // snapshot the DOM.
      setTimeout(done, 350);
    }
  });
  observer.observe(root, { childList: true, subtree: true });

  // Hard timeout regardless of observer state.
  setTimeout(function () {
    observer.disconnect();
    done();
  }, 8000);
})();
"##;
