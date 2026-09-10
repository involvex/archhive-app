# GitHub Pages deployment

Deploy the ArcHive **frontend SPA** to GitHub Pages so it is accessible from any browser on your LAN. Downloads, library scanning, and cookies remain on the **desktop ArcHive app** — the Pages site is a thin UI client that connects to the desktop over the LAN REST API (port 8787).

## What runs on Pages

| Component               | Location        | Notes                                    |
| ----------------------- | --------------- | ---------------------------------------- |
| React SPA (HTML/JS/CSS) | GitHub Pages    | Fully static, no backend                 |
| yt-dlp / gallery-dl     | Desktop ArcHive | Runs on the host, not in the browser     |
| SQLite library          | Desktop ArcHive | Queried via LAN API                      |
| Video streams           | Desktop ArcHive | Proxied through `/api/scenes/{id}/media` |

> **Routing note:** TanStack Router must be told the site's subdirectory prefix via a build-time `basepath` constant (`__GHPAGES_BASEPATH__`). Vite's `import.meta.env.BASE_URL` is **not** inlined in minified output, so the workflow uses a `define()` replacement instead. See [Build configuration](#router-basepath) below.

No server-side runtime is needed on Pages. CORS is already permissive on the ArcHive LAN server (`CorsLayer::permissive()`), so cross-origin requests from `yourname.github.io` to `http://<desktop-ip>:8787` work out of the box.

## Prerequisites

1. Desktop ArcHive app installed and running
2. **Settings → LAN → Enable LAN server** (default port 8787)
3. Desktop and viewer on the **same LAN**
4. GitHub repo at `https://github.com/<username>/<repo-name>` (e.g. `archhive-app`)

## Build configuration (base path)

GitHub Pages serves the site under a subdirectory: `https://<username>.github.io/<repo-name>/`. Vite needs to know this prefix so asset URLs resolve correctly.

`vite.config.ts` reads `BASE_URL` from the environment:

```typescript
base: process.env.BASE_URL || "/",
```

- **Default (`"/"`)** — standalone desktop build, LAN server, etc.
- **GitHub Pages (`"/<repo-name>/"`)**: set `BASE_URL=/<repo-name>/` at build time.

## Router basepath

TanStack Router needs a `basepath` to prefix all client-side routes. Since Vite 8 does not inline `import.meta.env.BASE_URL` into the minified bundle, the workflow injects a build-time constant via Vite's `define` option:

**`vite.config.ts`:**

```typescript
define: {
  __GHPAGES_BASEPATH__: JSON.stringify(
    (process.env.BASE_URL || "/").replace(/\/$/, "") || "",
  ),
},
```

**`src/main.tsx`:**

```typescript
const router = createRouter({
  routeTree,
  basepath: __GHPAGES_BASEPATH__,
});
```

**`src/vite-env.d.ts`:**

```typescript
declare const __GHPAGES_BASEPATH__: string;
```

Result:

- Desktop/LAN build (`BASE_URL=/`): `basepath=""` — routes are `/`, `/browse`, `/library`, etc.
- GitHub Pages build (`BASE_URL=/archhive-app/`): `basepath="/archhive-app"` — routes are `/archhive-app/`, `/archhive-app/browse`, etc.

## Deploy via GitHub Actions (recommended)

Push to the `pages` branch to trigger deployment. The workflow builds the frontend and deploys the static output to the `gh-pages` branch automatically.

### One-time setup

1. In your repo, go to **Settings → Pages**.
2. Set **Source** to the `gh-pages` branch, root (`/`) folder.
3. Set **Build and deployment** → **Build type** to **Legacy** (recommended for external CI workflows).

### Workflow trigger

```bash
# From main, create or update the pages branch
git push origin main:pages
# Or just push to pages branch directly
git checkout -b pages
git push -u origin pages
```

The workflow (`.github/workflows/gh-pages.yml`) will:

1. Checkout the repo
2. Install Bun + dependencies
3. Generate plugin registry (`bun run plugins:generate`)
4. Typecheck (`bun run typecheck`)
5. Build with the correct base path (`BASE_URL=/<repo-name>/ bunx vite build`)
6. Upload to `gh-pages` branch via `peaceiris/actions-gh-pages`

The site will be live at `https://<username>.github.io/<repo-name>/` within a minute.

### Manual workflow dispatch

You can also trigger the build manually from the **Actions** tab → **GitHub Pages** workflow → **Run workflow**, choosing the branch to deploy from.

## Deploy manually (without Actions)

```bash
# 1. Build with the correct base path
BASE_URL=/<repo-name>/ bun run typecheck && BASE_URL=/<repo-name>/ bunx vite build

# 2. Deploy using ghp or the actions-gh-pages action
npx gh-pages -d dist -t '<repo-name>'
# or
# git checkout -b gh-pages
# cp -r dist/* .
# git add -A && git commit -m "Deploy to gh-pages" && git push -u origin gh-pages
```

Need the tool? `bun add -d gh-pages` or use `peaceiris/actions-gh-pages` in CI.

## Post-deploy configuration

When you open `https://<username>.github.io/<repo-name>/`, the app loads in **browser** runtime mode. It does not know your desktop LAN address until you configure it:

1. Open **Settings → Engine**
2. Select **Remote LAN**
3. Enter your desktop LAN URL: `http://<desktop-ip>:8787`
   - Find the IP on your desktop: `ipconfig` (Windows) or `ifconfig` (macOS/Linux)
   - Example: `http://192.168.178.69:8787`
4. **Settings → LAN → Copy web link** on the desktop app to get the token
5. Enter the token in the **Remote LAN token** field on the Pages site

The settings persist in **localStorage** under key `archhive-settings`, so you only need to do this once.

### Auto-connect from URL

You can pre-seed the connection by adding `?host=` and `?token=` URL parameters:

```
https://<username>.github.io/<repo-name>/?host=http://192.168.178.69:8787&token=your-lan-token
```

This is convenient for bookmarking on a specific machine.

## Streaming limitations

- **Video playback**: Library scenes stream from the desktop via `GET /api/scenes/{id}/media` (HTTP Range support). The browser must be able to reach the desktop LAN address.
- **Live streams** (Chaturbate, etc.): `GET /api/media/livestream` resolves on the desktop host; the stream URL is returned to the browser.
- **Downloads**: Not possible from the Pages UI — yt-dlp runs on the desktop. Use the desktop app or mobile APK to queue downloads.

## Troubleshooting

| Issue                                     | Fix                                                                                                                                             |
| ----------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Assets 404 (wrong `base`)                 | Ensure `BASE_URL=/<repo-name>/` was set at build time; check `dist/index.html` for correct asset paths                                          |
| "Cannot reach host" after connect         | Desktop ArcHive LAN server not running, or Windows Firewall blocking TCP 8787. See [lan-web.md](lan-web.md)                                     |
| 401 Unauthorized                          | Token mismatch — regenerate on desktop (Settings → LAN) and re-paste in Settings → Engine                                                       |
| mDNS not found                            | GitHub Pages is not on the LAN; enter the desktop IP manually (e.g. `http://192.168.1.69:8787`)                                                 |
| Videos won't play                         | Same-origin policy — the desktop must serve over HTTP on the LAN. HTTPS on GitHub Pages → HTTP on LAN is supported by the permissive CORS layer |
| "Folder picking requires the app runtime" | Expected — this action runs locally on desktop/mobile only, not from a browser                                                                  |

## Related

- [LAN web UI and streaming](lan-web.md) — browser access from the desktop itself
- [Android development and Remote LAN](mobile-android.md) — mobile app connecting to desktop
- [Releasing ArcHive](release.md) — version bump and GitHub Releases workflow
