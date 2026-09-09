# Sidecar binaries

Tauri bundles `yt-dlp` and `ffmpeg` from this folder. gallery-dl uses PATH fallback.

## Setup

**Windows:**

```powershell
.\scripts\setup-binaries.ps1
.\scripts\setup-binaries.ps1 -IncludeAndroid
```

**macOS / Linux:**

```bash
./scripts/setup-binaries.sh
./scripts/setup-binaries.sh --android
```

Expected sidecar name after download:
`yt-dlp-<TARGET_TRIPLE>[.exe]` (e.g. `yt-dlp-x86_64-pc-windows-msvc.exe`)

Run setup before `cargo build` or `bun run tauri build`.

**Android:** ffmpeg and ffprobe ARM64 binaries are bundled via `tauri.android.conf.json` for thumbnail extraction, duration probing, and media processing. Download with `-IncludeAndroid` / `--android`.

**iOS:** sidecars are not bundled. Mobile uses Remote LAN or standalone HTTP.

## Android binaries

| Binary | Purpose | Size |
|--------|---------|------|
| `ffmpeg-aarch64-linux-android` | Thumbnails, probing, format detection | ~104MB |
| `ffprobe-aarch64-linux-android` | Media metadata extraction | ~104MB |

These are statically-linked ARM64 builds from FFmpeg-Builds. They enable full library features (scanning, thumbnails, duration probing) on Android without requiring a desktop connection.

## Optional full download engine

For complete site support (all 15 adapters), install yt-dlp in Settings → Advanced. This downloads a Python-based yt-dlp binary to app data and enables full download parity with the desktop app.
