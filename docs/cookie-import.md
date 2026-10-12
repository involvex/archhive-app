# Cookie import for ArcHive

ArcHive stores cookies encrypted per site for browse and yt-dlp downloads.

## Recommended: Log in with browser (desktop + Android)

1. In **Settings → Cookies**, select the site (or open a cookie-required site in Browse).
2. Press **Log in with browser**. ArcHive opens the site in its own browser:
   - **Desktop:** a separate window. Log in, then press **Save cookies** in ArcHive (or just close the window).
   - **Android:** a full-screen browser with **Cancel** / **Save cookies** buttons at the top. The system back button navigates back inside the page.
3. The captured cookies, including HttpOnly session cookies that page scripts cannot read, are converted to Netscape format and stored in the vault (on the LAN host when the app is in Remote LAN mode).

Android does not let one app read another app's cookies (for example Chrome's), so the login has to happen inside ArcHive. Google sign-in may refuse embedded browsers; for YouTube, import cookies with Cookie-Editor instead.

Implementation: `src-tauri/src/cookie_login.rs` (desktop window + Android bridge) and `src-tauri/android-overlays/CookieBrowserPlugin.kt` (installed by `scripts/patch-android-ytdlp.ps1` / `.sh`).

## Cookie-Editor extension

1. Install [Cookie-Editor](https://chromewebstore.google.com/detail/cookie-editor/hlkenndednhfkekhgcdicdfddnkalmdm) (Chrome, Edge, Brave) or the Firefox equivalent.
2. Log in on the target site (PornHub, xHamster, etc.).
3. Open Cookie-Editor → **Export** → **JSON**.
4. In ArcHive **Settings → Cookies**, select the site and paste JSON → **Import JSON**.

ArcHive converts JSON to Netscape format and saves it in the encrypted vault.

## Bookmarklet (login shortcut)

1. Select a site in Settings → Cookies.
2. Click **Copy login bookmarklet**.
3. Create a browser bookmark whose URL is the copied `javascript:...` snippet.
4. Click the bookmark to open the site login page, then export cookies via Cookie-Editor.

## Manual Netscape paste

Export or build a Netscape cookie file and paste into the Netscape textarea → **Save Netscape Cookies**.

## Supported sites (cookie-required)

| Site ID  | Domains filtered on import  |
| -------- | --------------------------- |
| pornhub  | pornhub.com                 |
| xhamster | xhamster.com, xhamster.desi |
| xvideos  | xvideos.com                 |
| thisvid  | thisvid.com                 |

Lines exported as `#HttpOnly_<domain>` are treated as cookies, not comments.

## Security

- Cookies are encrypted at rest (AES-256-GCM).
- Plaintext cookie files are written only for yt-dlp `--cookies` usage under the app data directory.
- Do not share exported cookie files.
