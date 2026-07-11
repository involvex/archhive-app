# Sidecar binaries

Tauri bundles `yt-dlp` from this folder. ffmpeg and gallery-dl fall back to PATH.

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
