# LAN web UI and streaming

When **Settings → LAN → Enable LAN server** (desktop) or **Share library on Wi-Fi** (Android) is on, ArcHive serves the built frontend and API on port **8787** (default).

## Hosts

| Host        | How to enable                                                               | Notes                                                            |
| ----------- | --------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| **Desktop** | Settings → LAN → Enable LAN server                                          | Optional auth; Windows firewall must allow TCP 8787              |
| **Android** | Settings → Engine → **Local**, then Settings → LAN → Share library on Wi-Fi | Token **always** required; persistent notification while sharing |

iOS does not host. The phone still runs a `127.0.0.1` media proxy for the in-app player when sharing is off.

## URLs

| URL                               | What you get                                             |
| --------------------------------- | -------------------------------------------------------- |
| `http://<lan-ip>:8787/`           | ArcHive web app (same UI, via Remote LAN API)            |
| `http://<lan-ip>:8787/files`      | Directory browser under your configured **library path** |
| `http://<lan-ip>:8787/api/health` | Health check (no auth)                                   |

Replace `<lan-ip>` with the host machine's Wi-Fi address (e.g. `192.168.178.69`). Settings → LAN → **Copy web link** includes the token when auth is required.

## Phone / tablet browser (no APK)

1. Enable LAN sharing on the desktop app or an Android phone (Local engine).
2. Copy the web link from Settings → LAN (or build `http://<ip>:8787/?token=<token>`).
3. Open in Chrome/Safari on the **same Wi-Fi**.
4. Browse library scenes and play videos, or open **Files** for folder-style browsing.

The SPA auto-configures `remote_host` to the same origin when loaded from `:8787`.

Guest networks with client isolation cannot reach the host. Mobile data is not a LAN.

## Mobile app as client (APK)

Use **Settings → Engine → Remote LAN** with `http://<host-ip>:8787` and the LAN token (desktop or another Android host). Video playback in the library uses the same streaming endpoints as the browser.

## Android as host

1. Engine → **On-device (this phone)**.
2. Settings → LAN → **Share library on Wi-Fi**.
3. Allow the notification (keeps the process alive under Doze).
4. Copy web link / show QR; open on another device on the same Wi-Fi.
5. Turn the switch off when finished — the app returns to the loopback-only media server.

Battery cost is real while the notification is up. Force-stopping the app ends sharing.

## Video streaming

- **Library scenes:** `GET /api/scenes/{id}/media` (HTTP Range, auth via Bearer or `?token=`)
- **Files browser:** `GET /api/files/stream?path=<relative-path>`

Supported in-browser formats depend on the device codec (MP4/WebM usually work; MKV may not).

## Vaulted media

Scenes in the **media vault** live outside the library root in app-private storage, so
the LAN server treats them as invisible by default:

| Condition                                                        | `GET /api/scenes/{id}`, `/thumb`, `/media`                  |
| ---------------------------------------------------------------- | ----------------------------------------------------------- |
| Vault locked                                                     | `404` (a locked vault must not confirm that a scene exists) |
| Vault unlocked, **Expose vaulted items over the LAN server** off | `404`                                                       |
| Vault unlocked, exposure on                                      | Served with the usual bearer token                          |

`GET /api/files` and `/api/files/stream` never list or serve the vault directory, whatever
the exposure setting — the folder browser is scoped to the library path.

Exposure is off by default on purpose: a bearer token is not a biometric check, and the
LAN token can end up in a browser history, a QR code screenshot, or a shared link. See
`docs/vault.md`.

## Firewall (desktop)

Allow inbound **TCP 8787** on the desktop PC (Windows Defender Firewall).

## Not the Vite dev server

Port **1420** is Vite dev UI only during `bun run tauri dev`. Remote LAN and browser access use **8787**.
