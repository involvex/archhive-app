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

Install [yt-dlp](https://github.com/yt-dlp/yt-dlp) on your PATH. Optional: `ffmpeg`, `gallery-dl`.

## Scripts

| Command             | Description               |
| ------------------- | ------------------------- |
| `bun run dev`       | Vite dev server           |
| `bun run tauri dev` | Desktop app               |
| `bun run build`     | Production frontend build |
| `bun run lint`      | ESLint                    |
| `bun run format`    | Prettier write            |

## Architecture

See [Plan.md](Plan.md) for full architecture, site adapter docs in [docs/sites.md](docs/sites.md).

## LAN Mode

1. Open **Settings → LAN**
2. Enable server (default port 8787)
3. Copy API token
4. On mobile, set Engine mode to **Remote LAN** with `http://<desktop-ip>:8787`

## License

Private / TBD
