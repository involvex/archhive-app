#!/usr/bin/env bash
# Regenerate Android project for CI/Linux (and local Unix shells).
# Windows devs can keep using scripts/android-regen.ps1.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GEN_ANDROID="$ROOT/src-tauri/gen/android"

if [[ -d "$GEN_ANDROID" ]]; then
  echo "Removing stale src-tauri/gen/android ..."
  rm -rf "$GEN_ANDROID"
fi

cd "$ROOT"

if [[ -z "${NDK_HOME:-}" ]]; then
  echo "NDK_HOME is not set. Install the NDK and export NDK_HOME before running android init." >&2
  exit 1
fi

echo "Running tauri android init (NDK_HOME=$NDK_HOME) ..."
bun run tauri android init

GRADLE_PROPS="$GEN_ANDROID/gradle.properties"
if [[ ! -f "$GRADLE_PROPS" ]]; then
  echo "gradle.properties not found after android init." >&2
  exit 1
fi

if command -v pwsh >/dev/null 2>&1; then
  pwsh -NoProfile -File "$ROOT/scripts/patch-android-lan.ps1"
else
  bash "$ROOT/scripts/patch-android-lan.sh"
fi

ICON="$ROOT/assets/branding/icon-source.png"
ICON_SQUARE="$ROOT/assets/branding/icon-square.png"
if [[ -f "$ICON" ]]; then
  echo "Applying launcher icons from $ICON ..."
  bun run tauri icon "$ICON"
elif [[ -f "$ICON_SQUARE" ]]; then
  echo "Applying launcher icons from $ICON_SQUARE ..."
  bun run tauri icon "$ICON_SQUARE"
else
  echo "No assets/branding/icon-source.png — skip tauri icon."
fi

echo "Android project regenerated."
