#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="$ROOT/src-tauri/binaries"
mkdir -p "$BIN_DIR"

ARCH="${1:-$(rustc -vV | awk '/host:/ {print $2}')}"
EXT=""
if [[ "$ARCH" == *windows* ]]; then EXT=".exe"; fi

# Reject HTML/error pages that curl still saved as a "successful" download.
assert_xz_archive() {
  local path="$1"
  local label="${2:-archive}"
  if [[ ! -s "$path" ]]; then
    echo "error: $label download is empty ($path)" >&2
    return 1
  fi
  # xz magic: FD 37 7A 58 5A 00
  local magic
  magic="$(od -An -N6 -tx1 "$path" | tr -d ' \n')"
  if [[ "$magic" != "fd377a585a00" ]]; then
    echo "error: $label is not a valid .tar.xz (got magic '$magic'; first bytes follow)" >&2
    od -An -N64 -tx1c "$path" >&2 || true
    return 1
  fi
}

extract_btbn_ffmpeg() {
  local tar_path="$1"
  local dest="$2"
  local tmp_dir
  tmp_dir="$(mktemp -d)"
  tar -xJf "$tar_path" -C "$tmp_dir"
  local bin_dir
  bin_dir="$(find "$tmp_dir" -type d -name bin | head -n 1)"
  if [[ -z "$bin_dir" || ! -f "$bin_dir/ffmpeg" ]]; then
    echo "error: ffmpeg binary not found inside BtbN archive" >&2
    rm -rf "$tmp_dir"
    return 1
  fi
  cp "$bin_dir/ffmpeg" "$dest"
  rm -rf "$tmp_dir"
}

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
    # Prefer BtbN GitHub releases (same mirror as the Android path). The old
    # johnvansickle.com URL intermittently returns non-xz payloads that break CI.
    if [[ "$ARCH" == *aarch64* || "$ARCH" == *arm64* ]]; then
      FFMPEG_URL="https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linuxarm64-gpl.tar.xz"
    else
      FFMPEG_URL="https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linux64-gpl.tar.xz"
    fi
    echo "Downloading ffmpeg (Linux static from BtbN)..."
    TAR_PATH="$(mktemp).tar.xz"
    curl -fsSL "$FFMPEG_URL" -o "$TAR_PATH"
    assert_xz_archive "$TAR_PATH" "ffmpeg"
    extract_btbn_ffmpeg "$TAR_PATH" "$FFMPEG_OUT"
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

if [[ "${2:-}" == "--android" ]]; then
  echo "Downloading Android ARM64 ffmpeg/ffprobe..."
  ANDROID_TAR="$(mktemp).tar.xz"
  curl -fsSL "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linuxarm64-gpl.tar.xz" -o "$ANDROID_TAR"
  assert_xz_archive "$ANDROID_TAR" "Android ffmpeg"
  ANDROID_DIR="$(mktemp -d)"
  tar -xJf "$ANDROID_TAR" -C "$ANDROID_DIR"
  ANDROID_BIN_DIR="$(find "$ANDROID_DIR" -type d -name bin | head -n 1)"
  if [[ -n "$ANDROID_BIN_DIR" ]]; then
    cp "$ANDROID_BIN_DIR/ffmpeg" "$BIN_DIR/ffmpeg-aarch64-linux-android"
    cp "$ANDROID_BIN_DIR/ffprobe" "$BIN_DIR/ffprobe-aarch64-linux-android"
    chmod +x "$BIN_DIR/ffmpeg-aarch64-linux-android" "$BIN_DIR/ffprobe-aarch64-linux-android"
    echo "Android ffmpeg/ffprobe installed to $BIN_DIR"
  else
    echo "Warning: Android ffmpeg download failed"
  fi
  rm -rf "$ANDROID_DIR" "$ANDROID_TAR"
fi

echo "gallery-dl: install on PATH (pip install gallery-dl) or extend this script."
echo "Done."
