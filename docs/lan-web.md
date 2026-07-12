# LAN web UI and streaming

When **Settings → LAN → Enable LAN server** is on, the desktop app serves the built frontend and API on port **8787** (default).

## URLs

| URL                                  | What you get                                             |
| ------------------------------------ | -------------------------------------------------------- |
| `http://<pc-lan-ip>:8787/`           | ArcHive web app (same UI as desktop, via Remote LAN API) |
| `http://<pc-lan-ip>:8787/files`      | Directory browser under your configured **library path** |
| `http://<pc-lan-ip>:8787/api/health` | Health check (no auth)                                   |

Replace `<pc-lan-ip>` with your desktop machine's LAN address (e.g. `192.168.178.69`). Settings → LAN → **Copy web link** includes the token when auth is required.

## Phone browser (no APK)

1. Enable LAN on the desktop app.
2. Copy the web link from Settings → LAN (or build `http://<ip>:8787/?token=<token>`).
3. Open in Chrome/Safari on the same Wi‑Fi.
4. Browse library scenes and play videos, or open **Files** for folder-style browsing.

The SPA auto-configures `remote_host` to the same origin when loaded from `:8787`.

## Mobile app (APK)

Use **Settings → Engine → Remote LAN** with `http://<pc-ip>:8787` and the LAN token. Video playback in the library uses the same streaming endpoints as the browser.

## Video streaming

- **Library scenes:** `GET /api/scenes/{id}/media` (HTTP Range, auth via Bearer or `?token=`)
- **Files browser:** `GET /api/files/stream?path=<relative-path>`

Supported in-browser formats depend on the device codec (MP4/WebM usually work; MKV may not).

## Firewall

Allow inbound **TCP 8787** on the desktop PC (Windows Defender Firewall).

## Not the Vite dev server

Port **1420** is Vite dev UI only during `bun run tauri dev`. Remote LAN and browser access use **8787**.
