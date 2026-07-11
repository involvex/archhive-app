# ArcHive App — Architecture Plan

Cross-platform desktop and mobile media browser, downloader, and personal library manager.

## Product Vision

| Capability                        | Reference                                               | Implementation                                       |
| --------------------------------- | ------------------------------------------------------- | ---------------------------------------------------- |
| Multi-site browse/search/download | [SCrawler](https://github.com/AAndyProgram/SCrawler)    | Rust Site Adapter registry + yt-dlp / gallery-dl     |
| Custom tube sites (tags, models)  | e.g. ThotHub                                            | Configurable adapters with URL pattern routing       |
| Library, performers, tags         | [Stash](https://github.com/stashapp/stash) (local only) | SQLite + scanner + auto-tagger                       |
| Modern UI                         | —                                                       | React 19 + TanStack Router + Tailwind (dark default) |
| LAN access                        | —                                                       | Embedded Axum REST API + mDNS                        |

## Stack

| Layer           | Technology                                               |
| --------------- | -------------------------------------------------------- |
| Shell           | Tauri v2 (Windows, macOS, Linux, iOS, Android)           |
| Backend         | Rust (IPC, downloads, DB, LAN server)                    |
| Frontend        | React + Vite + TypeScript + TanStack Router              |
| Styling         | Tailwind CSS v4 + shadcn-style components                |
| Package manager | Bun                                                      |
| Database        | SQLite (rusqlite) + FTS5                                 |
| Downloads       | yt-dlp (primary), gallery-dl, ffmpeg via PATH or sidecar |

## Runtime Modes

| Mode           | Platform         | Capability                        |
| -------------- | ---------------- | --------------------------------- |
| **Local**      | Desktop          | Full yt-dlp / gallery-dl / ffmpeg |
| **Standalone** | Mobile           | YouTube + direct media URLs only  |
| **Remote LAN** | Mobile → desktop | Full parity via REST API          |

Configure in **Settings → Engine**.

## Project Structure

```
scrawler-app/
├── src/                    # React UI
│   ├── routes/             # TanStack file routes
│   ├── components/         # UI + layout
│   └── lib/api/            # IPC + LAN client abstraction
├── src-tauri/
│   ├── src/
│   │   ├── commands.rs     # Tauri IPC
│   │   ├── server/         # Axum LAN server
│   │   ├── sites/          # Site adapters + yt-dlp runner
│   │   ├── downloads/      # Job queue
│   │   ├── library/        # Scanner + auto-tag
│   │   └── db/             # SQLite
│   └── migrations/
└── docs/
```

## Site Adapters

Built-in adapters:

- **thothub** — tag, model, search browse → yt-dlp download
- **pornhub**, **xhamster**, **xvideos** — browse + yt-dlp (cookies recommended)
- **reddit**, **redgifs** — browse + yt-dlp / gallery-dl
- **youtube**, **tiktok**, **twitter**, **thisvid** — generic yt-dlp

Add custom sites by implementing `SiteAdapter` in `src-tauri/src/sites/adapters/`.

## Library Model

- **Scene** — title, path, thumb, source URL, performers, tags
- **Performer** — name, aliases, scene count
- **Tag** — hierarchical tags with scene counts
- **DownloadJob** — queue with progress events

Auto-tagging: filename regex rules + metadata from download jobs.

## LAN Server

Enable in **Settings → LAN**. Endpoints:

- `GET /api/health`
- `GET /api/sites`, `GET /api/sites/{id}/browse`
- `GET|POST /api/downloads`
- `GET /api/scenes`, `/api/performers`, `/api/tags`

Bearer token required (except `/api/health`). Advertises `_archhive._tcp` via mDNS.

## Development

```bash
bun install
bun run tauri dev
```

**Requirements:** `yt-dlp` and optionally `ffmpeg` / `gallery-dl` on PATH.

```bash
bun run build          # frontend
bun run lint           # eslint
bun run format:check   # prettier
cd src-tauri && cargo test
```

## Implementation Phases

1. **Foundation** — Tauri + React + SQLite ✓
2. **Downloads** — yt-dlp queue + progress ✓
3. **Browse** — Site adapters + ThotHub ✓
4. **Library** — Scenes, performers, tags, scanner ✓
5. **LAN + mobile** — Axum server, dual engine modes ✓
6. **Parity** — PornHub, xHamster, XVIDEOS, Reddit, RedGifs ✓
7. **Polish** — Cookie vault, phash, batch tag editor (future)

## References

- [SCrawler](https://github.com/AAndyProgram/SCrawler)
- [Stash](https://github.com/stashapp/stash)
- [yt-dlp](https://github.com/yt-dlp/yt-dlp)
- [gallery-dl](https://github.com/mikf/gallery-dl)
