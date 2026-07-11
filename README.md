# Scrawler

Cross-platform media browser, downloader, and personal library — inspired by [SCrawler](https://github.com/AAndyProgram/SCrawler) and [Stash](https://github.com/stashapp/stash).

## Features

- Browse and download from multiple sites (ThotHub, PornHub, xHamster, XVIDEOS, Reddit, RedGifs, YouTube, TikTok, and more)
- Paste-any-URL download via yt-dlp
- Local library with scenes, performers, tags, and FTS search
- Auto-tagging from filenames and download metadata
- Download queue with live progress
- Optional LAN web server for mobile full-parity access
- Mobile standalone mode for YouTube and direct media
- Dark-first responsive UI (desktop + mobile)

## Quick Start

```bash
bun install
bun run tauri dev
```

Install [yt-dlp](https://github.com/yt-dlp/yt-dlp) on your PATH, or bundle sidecars:

```bash
# Windows
.\scripts\setup-binaries.ps1

# macOS / Linux
./scripts/setup-binaries.sh
```

Optional: `gallery-dl` on PATH. ffmpeg is bundled by the setup script.

## Scripts

| Command                     | Description                       |
| --------------------------- | --------------------------------- |
| `bun run dev`               | Vite dev server                   |
| `bun run tauri dev`         | Desktop app                       |
| `bun run tauri:android:dev` | Android app (device/emulator)     |
| `bun run setup:binaries`    | Download yt-dlp + ffmpeg sidecars |
| `bun run build`             | Production frontend build         |
| `bun run lint`              | ESLint                            |
| `bun run format`            | Prettier write                    |

## Architecture

See [Plan.md](Plan.md) for full architecture, site adapter docs in [docs/sites.md](docs/sites.md).

## LAN Mode

1. Open **Settings → LAN**
2. Enable server (default port 8787)
3. Copy API token
4. On mobile, set Engine mode to **Remote LAN** with `http://<desktop-ip>:8787`

See [docs/mobile-android.md](docs/mobile-android.md) for emulator (`10.0.2.2`) and device testing.

## Cookie import

See [docs/cookie-import.md](docs/cookie-import.md) for Cookie-Editor JSON import and bookmarklet workflow.

## License

Private / TBD
