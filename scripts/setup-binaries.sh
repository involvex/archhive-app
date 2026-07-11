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

FFMPEG_OUT="$BIN_DIR/ffmpeg-$ARCH$EXT"
OS="$(uname -s)"
case "$OS" in
  Darwin)
    echo "Downloading ffmpeg (macOS)..."
    FFMPEG_URL="https://evermeet.cx/ffmpeg/getrelease/ffmpeg/zip"
    ZIP_PATH="$(mktemp).zip"
    curl -fsSL "$FFMPEG_URL" -o "$ZIP_PATH"
    unzip -qo "$ZIP_PATH" -d "$BIN_DIR"
    mv "$BIN_DIR/ffmpeg" "$FFMPEG_OUT"
    rm -f "$ZIP_PATH"
    ;;
  Linux)
    echo "Downloading ffmpeg (Linux static)..."
    FFMPEG_URL="https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz"
    TAR_PATH="$(mktemp).tar.xz"
    curl -fsSL "$FFMPEG_URL" -o "$TAR_PATH"
    tar -xJf "$TAR_PATH" -C "$BIN_DIR" --strip-components=1 --wildcards '*/ffmpeg'
    mv "$BIN_DIR/ffmpeg" "$FFMPEG_OUT"
    rm -f "$TAR_PATH"
    ;;
  *)
    echo "Unsupported OS for automatic ffmpeg download: $OS"
    echo "Install ffmpeg on PATH or copy to $FFMPEG_OUT"
    ;;
esac

if [[ -f "$FFMPEG_OUT" ]]; then
  chmod +x "$FFMPEG_OUT"
fi

echo "gallery-dl: install on PATH (pip install gallery-dl) or extend this script."
echo "Done."
