# Sidecar binaries

Tauri bundles `yt-dlp` and `ffmpeg` from this folder. gallery-dl uses PATH fallback.

## Setup

**Windows:**

```powershell
.\scripts\setup-binaries.ps1
```

**macOS / Linux:**

```bash
./scripts/setup-binaries.sh
```

Expected sidecar name after download:
`yt-dlp-<TARGET_TRIPLE>[.exe]` (e.g. `yt-dlp-x86_64-pc-windows-msvc.exe`)

Run setup before `cargo build` or `bun run tauri build`.
