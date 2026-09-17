# ArcHive — Android & Mobile Improvement Plan

> **Updated**: 2026-09-17 (post-review rewrite)  
> **Scope**: Accurate status vs codebase + remaining work  
> **Do not treat older drafts as current-state** — Local browsing and on-device library already work

---

## Executive Summary

ArcHive’s Android path is **usable in Local mode** (on-device SQLite, site browse via Rust adapters + youtubedl-android, loopback media streaming). Remote LAN remains the path for full desktop parity (gallery-dl sites, Chaturbate webview listing fallbacks). Phase A/B hardening (install guards, patch validate, DataSaver, browse cache, FG+WM resume) is in place; remaining gaps are gallery-dl, Chaturbate reliability, and deferred PWA/iOS.

---

## I. Accurate Current State

### A. Engine modes (mobile)

| Mode                | Status  | Capability                                                                                 |
| ------------------- | ------- | ------------------------------------------------------------------------------------------ |
| **Local** (default) | Done    | On-device DB, browse via adapters, yt-dlp bridge, loopback `:8787` media/thumbs            |
| **Standalone**      | Partial | Same backend as Local for library; `resolve_standalone()` still YouTube + direct URLs only |
| **Remote LAN**      | Done    | Full desktop API parity when host + token configured                                       |

Bootstrap: `bootstrap_mobile_settings` demotes `RemoteLan` → `Local` only when `remote_host` is empty ([`src-tauri/src/lib.rs`](src-tauri/src/lib.rs)).

### B. Android build pipeline

| Component                                 | Status           | Notes                                                                                     |
| ----------------------------------------- | ---------------- | ----------------------------------------------------------------------------------------- |
| `scripts/android-dev.ps1`                 | Done             | AVD, `TAURI_DEV_HOST`, separate cargo target                                              |
| `scripts/android-regen.ps1` / `.sh`       | Done             | Regen + patches + icon                                                                    |
| `scripts/patch-android-lan.ps1` / `.sh`   | Done             | Cleartext, mDNS, media perms, PiP                                                         |
| `scripts/patch-android-ytdlp.ps1` / `.sh` | Done             | `YtDlpPlugin.kt`, AAR deps, ProGuard                                                      |
| `scripts/apply-android-patches.ps1`       | Done             | Single entry that runs lan + ytdlp                                                        |
| `scripts/validate-android-build.ps1`      | Done             | Preflight checks for `gen/android` overlays                                               |
| `src-tauri/android-overlays/*`            | Done             | Kotlin plugin + ProGuard keep rules                                                       |
| `capabilities/mobile.json`                | Done             | Mobile IPC / remote URLs                                                                  |
| `tauri.android.conf.json`                 | Partial          | Exists; merge overlay for `lan-ui` only — SDK/permissions live in patched Gradle/Manifest |
| `gen/android` gitignored                  | Done (by design) | Must regen + re-patch after `tauri android init`                                          |
| APK scripts auto-patch                    | Done             | `prebuild:apk` / `build:apk*` run ytdlp (+ apply) patches                                 |
| Android CI                                | Done             | `.github/workflows/release.yml` `build-android`                                           |

### C. Mobile Rust / plugins

| File                                        | Status  | Notes                                                         |
| ------------------------------------------- | ------- | ------------------------------------------------------------- |
| `mobile/ytdlp_bridge.rs` + `YtDlpPlugin.kt` | Done    | yt-dlp + FFmpeg via youtubedl-android                         |
| `mobile/standalone.rs`                      | Partial | YouTube + direct URLs for that helper only                    |
| `mobile/binary_installer.rs`                | Done    | Hard-fails on Android (use engine update, not Linux sidecars) |
| Startup deletes stale Linux bins            | Done    | `ensure_sidecar_permissions` / cleanup on Android             |
| Loopback LAN server                         | Done    | Started on mobile for local media URLs                        |

### D. Frontend mobile

| Item                            | Status  | Path / notes                                                                    |
| ------------------------------- | ------- | ------------------------------------------------------------------------------- |
| Bottom nav                      | Done    | Home, Browse, Library, Live, Downloads, Settings — `AppShell.tsx`               |
| IPC ↔ Remote LAN                | Done    | `src/lib/runtime.ts`, `src/lib/api/client.ts`                                   |
| Pull-to-refresh                 | Done    | `src/lib/hooks/usePullToRefresh.ts`                                             |
| Network / Wi‑Fi-only downloads  | Done    | `download_on_wifi_only`, `useNetworkMonitor.ts`                                 |
| Device cookie vault (Local)     | Done    | `shouldUseDeviceCookieVault()` in `client.ts`                                   |
| Settings backup share sheet     | Done    | Settings → Diagnostics                                                          |
| PiP button                      | Partial | `HlsVideoPlayer.tsx` + manifest PiP                                             |
| Diagnostics                     | Done    | `get_diagnostics` + Settings UI                                                 |
| Download complete notifications | Done    | Channels + complete/fail + FG progress keep-alive                               |
| Offline browse cache            | Done    | SQLite `browse_cache` + stale UI banner                                         |
| Background download survival    | Partial | FG service while Active; WorkManager wakes/re-queues — not mid-transfer offload |
| Full DataSaverMode              | Done    | `data_saver` Off / OnMetered / Always → 480p cap                                |
| PWA / iOS                       | Todo    | Deferred                                                                        |

### E. Site adapters

**16** registered adapters in [`src-tauri/src/sites/registry.rs`](src-tauri/src/sites/registry.rs) (Chaturbate, Stripchat, ThotHub, Pornhub, YouPorn, XNXX, xHamster, XVIDEOS, Reddit, RedGifs, Custom, + GenericYtDlp: YouTube/TikTok/Instagram/Twitter/ThisVid).

Limits on Android Local:

- **gallery-dl** not available (sidecar skipped)
- Chaturbate/Stripchat **webview listing** is `#[cfg(desktop)]` — HTTP/API paths used on mobile; cookies often required

---

## II. Completed (no longer open work)

| Item                         | Evidence                                     |
| ---------------------------- | -------------------------------------------- |
| Local browse without desktop | IPC `browse` + adapters + yt-dlp bridge      |
| On-device library / FTS      | SQLite via `localInvoke` when not Remote LAN |
| Download queue resume        | `DownloadManager::new` re-queues Pending     |
| Wi‑Fi-only downloads         | `download_on_wifi_only`                      |
| Loopback media streaming     | `ensure_loopback_server` + `mediaUrl.ts`     |
| youtubedl-android overlay    | patches + APK prebuild                       |
| Android release CI           | `release.yml`                                |
| PiP UI + manifest flag       | HlsVideoPlayer + patch-android-lan           |
| Installer Android guard      | `binary_installer.rs` cfg(android)           |
| DataSaverMode                | `data_saver` + client `navigator.connection` |
| Browse cache                 | `browse_cache` table + UI banner             |
| FG service + WM resume       | overlays + `ytdlp_bridge` keep-alive         |
| youtubedl-android pin        | `0.18.1` in patch-android-ytdlp scripts      |

---

## III. Remaining roadmap

### Phase A — Build hardening (priority)

| ID  | Item                                                            | Status | Effort |
| --- | --------------------------------------------------------------- | ------ | ------ |
| A1  | Hard-fail `install_yt_dlp` / `install_gallery_dl` on Android    | Done   | Low    |
| A2  | `apply-android-patches.ps1`                                     | Done   | Low    |
| A3  | `validate-android-build.ps1`                                    | Done   | Medium |
| A4  | Document that SDK/permissions are Manifest/Gradle after patches | Done   | Low    |
| A5  | Gradle caching / faster APK iteration                           | Todo   | Medium |

### Phase B — Mobile UX gaps

| ID  | Item                                                                   | Status  | Effort |
| --- | ---------------------------------------------------------------------- | ------- | ------ |
| B1  | Background downloads (FG keep-alive + WorkManager resume)              | Partial | High   |
| B2  | Offline browse cache (for Remote LAN offline / stale listings)         | Done    | High   |
| B3  | DataSaverMode (cap quality on metered)                                 | Done    | Medium |
| B4  | gallery-dl strategy on Android or clearer UI gating                    | Todo    | Medium |
| B5  | Chaturbate/Stripchat mobile listing reliability (cookies / no webview) | Partial | Medium |

### Phase C — Platform expansion (deferred)

| ID  | Item                                       | Status | Effort |
| --- | ------------------------------------------ | ------ | ------ |
| C1  | PWA manifest for browser LAN UI            | Todo   | Low    |
| C2  | iOS (Remote LAN–only recommended)          | Todo   | High   |
| C3  | Deep / universal links                     | Todo   | Medium |
| C4  | Mobile-optimized watchlist background poll | Todo   | Medium |

---

## IV. Architecture notes

```
Mobile Local:
  React → invoke → Rust adapters / DB / DownloadManager
                 → ytdlp_bridge (Kotlin) → yt-dlp / FFmpeg
                 → loopback Axum → scene/thumb/stream URLs

Mobile Remote LAN:
  React → fetch → desktop :8787 → full desktop stack
```

`tauri.android.conf.json` remains a **merge overlay** (`bundle.resources` → `lan-ui`). Do not invent unsupported SDK keys without verifying the Tauri v2 schema; permissions are applied by `patch-android-lan*.ps1|.sh`.

---

## V. Validation

```powershell
# After tauri android init / regen:
bun run android:regen   # or: .\scripts\apply-android-patches.ps1
.\scripts\validate-android-build.ps1

bun run build:apk:fast  # patches + typecheck path per package.json
```

Manual: Local browse (Pornhub/YouTube), library playback via loopback, Remote LAN connect, Settings diagnostics.

---

## VI. Risks (unchanged)

| Risk                                | Mitigation                                         |
| ----------------------------------- | -------------------------------------------------- |
| `tauri android init` wipes overlays | `apply-android-patches` + `validate-android-build` |
| youtubedl-android breaking upgrades | Pin version in patch script                        |
| Background DL battery               | WorkManager + constraints (when implemented)       |
| Stale offline browse cache          | TTL + UI badge (when implemented)                  |
