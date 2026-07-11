# Cookie import for ArcHive

ArcHive stores cookies encrypted per site for browse and yt-dlp downloads.

## Recommended: Cookie-Editor extension

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

## Security

- Cookies are encrypted at rest (AES-256-GCM).
- Plaintext cookie files are written only for yt-dlp `--cookies` usage under the app data directory.
- Do not share exported cookie files.
