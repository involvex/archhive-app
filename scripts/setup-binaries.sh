#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="$ROOT/src-tauri/binaries"
mkdir -p "$BIN_DIR"

ARCH="${1:-$(rustc -vV | awk '/host:/ {print $2}')}"
EXT=""
if [[ "$ARCH" == *windows* ]]; then EXT=".exe"; fi

YTDLP_OUT="$BIN_DIR/yt-dlp-$ARCH$EXT"
echo "Downloading yt-dlp to $YTDLP_OUT..."
curl -fsSL "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp" -o "$YTDLP_OUT"
chmod +x "$YTDLP_OUT"

echo "ffmpeg and gallery-dl: install on PATH or extend this script for your platform."
echo "Done."
