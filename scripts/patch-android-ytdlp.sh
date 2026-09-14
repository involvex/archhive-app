#!/usr/bin/env bash
# Patches generated Android project with YtDlpPlugin, youtubedl-android deps, and ProGuard keeps.
# Run after `tauri android init` / android:regen (gen/android is gitignored).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GEN_ANDROID="${1:-$ROOT/src-tauri/gen/android}"
OVERLAYS="$ROOT/src-tauri/android-overlays"

GRADLE="$GEN_ANDROID/app/build.gradle.kts"
PROGUARD="$GEN_ANDROID/app/proguard-rules.pro"
PLUGIN_DEST_DIR="$GEN_ANDROID/app/src/main/java/com/archhive/app"
PLUGIN_DEST="$PLUGIN_DEST_DIR/YtDlpPlugin.kt"
PLUGIN_SRC="$OVERLAYS/YtDlpPlugin.kt"
PROGUARD_OVERLAY="$OVERLAYS/proguard-ytdlp.pro"

if [[ ! -f "$GRADLE" ]]; then
  echo "Android project not found at $GEN_ANDROID. Run: bun run tauri android init" >&2
  exit 1
fi

if [[ ! -f "$PLUGIN_SRC" ]]; then
  echo "Missing overlay: $PLUGIN_SRC" >&2
  exit 1
fi

mkdir -p "$PLUGIN_DEST_DIR"
cp "$PLUGIN_SRC" "$PLUGIN_DEST"
echo "Installed YtDlpPlugin.kt -> $PLUGIN_DEST"

if ! grep -q 'youtubedl-android:library' "$GRADLE"; then
  python3 - "$GRADLE" <<'PY'
import sys
from pathlib import Path
path = Path(sys.argv[1])
text = path.read_text()
deps = '''    // youtubedl-android: bundles Python + yt-dlp for ARM64/ARMv7 Android
    implementation("io.github.junkfood02.youtubedl-android:library:0.18.1")
    implementation("io.github.junkfood02.youtubedl-android:ffmpeg:0.18.1")
'''
needle = "dependencies {"
idx = text.find(needle)
if idx < 0:
    raise SystemExit("Could not find dependencies block")
insert_at = idx + len(needle)
text = text[:insert_at] + "\n" + deps + text[insert_at:]
path.write_text(text)
print("Added youtubedl-android dependencies to build.gradle.kts")
PY
else
  echo "youtubedl-android Gradle deps already present."
fi

if [[ ! -f "$PROGUARD_OVERLAY" ]]; then
  echo "Missing overlay: $PROGUARD_OVERLAY" >&2
  exit 1
fi

if [[ -f "$PROGUARD" ]] && grep -q 'YtDlpPlugin' "$PROGUARD"; then
  echo "ProGuard YtDlp keep rules already present."
else
  {
    if [[ -f "$PROGUARD" ]]; then
      cat "$PROGUARD"
      printf '\n\n'
    fi
    cat "$PROGUARD_OVERLAY"
    printf '\n'
  } > "$PROGUARD.tmp"
  mv "$PROGUARD.tmp" "$PROGUARD"
  echo "Wrote ProGuard keep rules to $PROGUARD"
fi

echo "Android YtDlp overlay patch complete."

STALE_BINARIES="$GEN_ANDROID/app/src/main/assets/binaries"
if [[ -d "$STALE_BINARIES" ]]; then
  rm -rf "$STALE_BINARIES"
  echo "Removed stale assets/binaries (Linux ffmpeg sidecars)."
fi
