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

## Android: in-app capture (fastest)

On Android you can harvest a session without a browser or extension:

1. **Settings → Cookies**, select the site (every site is listed, whether or not it normally
   needs cookies — twitter, eporner, xnxx, xhamster, youporn and the rest).
2. **Catch cookies in-app** — ArcHive opens the site login page in a WebView **inside the
   app** (not the system browser: Android sandboxes another app's cookie store, so ArcHive
   must own the WebView to read the session).
3. Log in there, then tap **“Save cookies”** in the window header.

   Capture is **manual on purpose**. Auto-saving on the first cookie change misfired twice:
   Cloudflare/consent/analytics cookies land right after the page loads, and multi-step
   logins (e.g. Stripchat via the X/Twitter OAuth flow) set the _provider's_ cookies before
   you finish the site's own step — either case would have written a half-finished session
   over a good one.

   **Cancel**, Back and the 10 minute timeout save nothing. The status line tells you when
   the cookie set actually changed, so you know when **Save cookies** is worth pressing.

4. The harvest is written straight into the encrypted vault — no copy/paste.

> **ThotHub domain notes.** `thothub.tv` is a parked ad domain now (it redirects to a casino
> landing page). The real site is **`thethothub.com`** (mirrors: `thothub.to`, `thothub.org`).
> Its `/login/` path is only a fancybox _popup fragment_ — opened directly it renders as a
> bare, unstyled form, so the capture window opens the **homepage** and you log in from there.

Note: a WebView only exposes non-HttpOnly cookies. Sites that keep _all_ auth state in
HttpOnly cookies must use the DevTools / Cookie-Editor JSON import below (those see HttpOnly).

### Shared cookie JSON

If you share cookie JSON into ArcHive (Android share sheet), the app notices it is a cookie
export and offers a one-tap import dialog with the site pre-selected from the cookie domains.

## Termux handoff (Android, optional — custom fork)

**Optional.** ArcHive works fully without Termux; this only helps if you also drive `yt-dlp`
by hand in a terminal. The integration targets the custom fork
[involvex/termux-app](https://github.com/involvex/termux-app) (`com.involvex.termux_app`).

ArcHive can push a site's stored jar into Termux so your manual `yt-dlp` runs use the same
sessions:

1. Install the [Termux fork](https://github.com/involvex/termux-app) and open ArcHive once
   (so the RUN_COMMAND permission is granted).
2. Enable the external API in Termux: add `allow-external-apps = true` to
   `~/.termux/termux.properties` (Settings → Properties in some builds) and restart Termux.
3. **Settings → Cookies** → **→ Termux** next to a stored site.

ArcHive calls the fork's `RunCommandService` (app-shell runner) with cookies on stdin and
writes `~/arc-hive/<site>.cookies`. The fork this app targets is built with runtime package
`com.involvex.termux_app` (Java namespace `com.invapp`), action prefix
`com.involvex.termux_app.RUN_COMMAND`. If you rebuild Termux with a different package, update
`TermuxBridgePlugin.kt` (`scripts/patch-android-cookie.ps1` installs it from
`src-tauri/android-overlays/`) and re-run `bun run android:patches`.

Termux cannot read another app's cookie store, so this is a handoff _to_ Termux — it never
extracts cookies from an external browser.

## Recommended (desktop + Android): Cookie-Editor extension

1. Install [Cookie-Editor](https://chromewebstore.google.com/detail/cookie-editor/hlkenndednhfkekhgcdicdfddnkalmdm) (Chrome, Edge, Brave) or the Firefox equivalent.
2. Log in on the target site (PornHub, xHamster, etc.).
3. Open Cookie-Editor → **Export** → **JSON**.
4. In ArcHive **Settings → Cookies**, select the site and paste JSON → **Import JSON**.

ArcHive converts JSON to Netscape format and saves it in the encrypted vault.

## DevTools table (no extension)

F12 → **Application → Cookies** → select the domain → select the table rows → copy → paste
into **Settings → Cookies** → **Import DevTools table**.

## Bookmarklet (login shortcut)

1. Select a site in Settings → Cookies.
2. Click **Copy login bookmarklet**.
3. Create a browser bookmark whose URL is the copied `javascript:...` snippet.
4. Click the bookmark to open the site login page, then export cookies via Cookie-Editor.

## Manual Netscape paste

Export or build a Netscape cookie file and paste into the Netscape textarea → **Save Netscape Cookies**.

## Site cookie domains used on import

Every site is selectable in the Cookies tab (in-app capture needs a server-side login page,
which these all have). This table lists the cookie domains ArcHive keeps when importing or
capturing a jar.

| Site ID    | Domains filtered on import               |
| ---------- | ---------------------------------------- |
| pornhub    | pornhub.com                              |
| xhamster   | xhamster.com, xhamster.desi              |
| xvideos    | xvideos.com                              |
| thisvid    | thisvid.com                              |
| chaturbate | chaturbate.com                           |
| stripchat  | stripchat.com                            |
| thothub    | thethothub.com, thothub.to               |
| eporner    | eporner.com                              |
| youporn    | youporn.com                              |
| xnxx       | xnxx.com                                 |
| reddit     | reddit.com, redd.it                      |
| redgifs    | redgifs.com                              |
| twitter    | twitter.com, x.com                       |
| instagram  | instagram.com                            |
| tiktok     | tiktok.com                               |
| youtube    | youtube.com, googlevideo.com, google.com |

`thothub.tv` is intentionally absent — it is a parked ad domain now.

Lines exported as `#HttpOnly_<domain>` are treated as cookies, not comments.

## Security

- Cookies are encrypted at rest (AES-256-GCM).
- Plaintext cookie files are written only for yt-dlp `--cookies` usage under the app data directory.
- Do not share exported cookie files.
