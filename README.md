# ArcHive

Cross-platform media browser, downloader, and personal library — inspired by [SCrawler](https://github.com/AAndyProgram/SCrawler) and [Stash](https://github.com/stashapp/stash).

> **Development workspace:** `D:\repos\archhive-app` (canonical clone). Do not use `I:\dev\scrawler-app` — that path was retired due to slow disk I/O.

Browse sites, queue downloads to your library, organize scenes with performers and tags, and optionally control everything from your phone over LAN.

## Features

- Multi-site browse and download (ThotHub, PornHub, xHamster, XVIDEOS, Reddit, RedGifs, YouTube, TikTok, and more)
- Paste-any-URL download via yt-dlp / gallery-dl / direct HTTP (images)
- SQLite library with FTS search, performers, tags, duplicates
- Library scene edit, rename-on-disk, thumbnails (desktop + LAN)
- PornHub category browser with orientation filters and live count refresh
- Download queue with live progress
- LAN REST API + mDNS + **web UI on port 8787** (library, video playback, `/files` folder browser)
- TypeScript plugins via `plugins/` directory (see [docs/plugins.md](docs/plugins.md))

## Prerequisites

- [Bun](https://bun.sh)
- [Rust](https://rustup.rs) (for Tauri desktop/Android)
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) on PATH (or bundled sidecars)
- Optional: `gallery-dl`, Android SDK for mobile builds

## Quick start (desktop)

```bash
bun install
bun run setup:binaries   # Windows: yt-dlp + ffmpeg sidecars
bun run tauri dev
```

## Quick start (Android + Remote LAN)

1. Desktop: `bun run android:dev` (starts LAN server + emulator/device build)
2. Phone: Settings → Engine → **Remote LAN** → pick discovered host (`http://<pc-ip>:8787`)
3. **Phone browser (no app):** open `http://<pc-ip>:8787/?token=<lan-token>` — full web UI + `/files` folder streaming
4. Copy LAN token from desktop Settings → LAN if auth is required

See [docs/mobile-android.md](docs/mobile-android.md).

## Engine modes

| Mode           | Where            | Capability                            |
| -------------- | ---------------- | ------------------------------------- |
| **Local**      | Desktop          | Full yt-dlp, gallery-dl, library scan |
| **Remote LAN** | Mobile / browser | Full parity via desktop REST API      |
| **Standalone** | Mobile offline   | Direct URL resolve only               |

Configure in **Settings → Engine**.

## Scripts

| Command                             | Description                                                                                                           |
| ----------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| `bun run dev`                       | Vite dev server (1420)                                                                                                |
| `bun run tauri dev`                 | Desktop app                                                                                                           |
| `bun run tauri:android:dev`         | Android on device/emulator                                                                                            |
| `bun run android:dev`               | Windows helper: AVD + LAN auto-start                                                                                  |
| `bun run android:regen`             | Regenerate `gen/android` after identifier/icon change (runs `tauri icon` if `assets/branding/icon-source.png` exists) |
| `bun run build:apk`                 | Debug APK (aarch64)                                                                                                   |
| `bun run build:apk:release`         | Release APK (aarch64)                                                                                                 |
| `bun run build:desktop`             | Windows NSIS installer                                                                                                |
| `bun run release -- -Version X.Y.Z` | Bump version, tag `vX.Y.Z`, push (CI publishes GitHub Release)                                                        |
| `bun run setup:binaries`            | Download yt-dlp + ffmpeg sidecars                                                                                     |
| `bun run plugins:generate`          | Regenerate plugin registry from `plugins/`                                                                            |
| `bun run build`                     | Lint + typecheck + production frontend                                                                                |
| `bun run lint` / `format`           | ESLint / Prettier                                                                                                     |

## Plugins

Clone a plugin repo into `plugins/<name>/`, then:

```bash
bun run plugins:generate
bun run dev
```

Author guide: [docs/plugins.md](docs/plugins.md).

## Documentation

Full index: [docs/README.md](docs/README.md).

## Troubleshooting

- Android logcat / FrameInsert: [docs/troubleshooting-android.md](docs/troubleshooting-android.md)
- Cookies: [docs/cookie-import.md](docs/cookie-import.md)
- LAN / mobile: [docs/mobile-android.md](docs/mobile-android.md)

## Architecture

See [Plan.md](Plan.md) for stack, adapters, and LAN API.

## Security

Report vulnerabilities privately via [SECURITY.md](SECURITY.md) (GitHub Security Advisories preferred). Do not file public issues with exploit details.

## License

Private / TBD
