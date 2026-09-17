#!/usr/bin/env bash
# Patches generated Android project with YtDlpPlugin, youtubedl-android deps, and ProGuard keeps.
# Run after `tauri android init` / android:regen (gen/android is gitignored).
#
# Version pin: change YOUTUBE_DL_ANDROID_VERSION below when upgrading.
# Then: apply patches / android:regen, smoke YtDlpPlugin execute + FFmpeg.init,
# and update docs/mobile-android.md if the pin changes.

set -euo pipefail

# Pin both library + ffmpeg AARs to the same release. Bump together only.
YOUTUBE_DL_ANDROID_VERSION="0.18.1"
WORK_RUNTIME_KTX_VERSION="2.9.1"

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

if [[ -f "$OVERLAYS/DownloadForegroundService.kt" ]]; then
  cp "$OVERLAYS/DownloadForegroundService.kt" "$PLUGIN_DEST_DIR/DownloadForegroundService.kt"
  echo "Installed DownloadForegroundService.kt"
fi
if [[ -f "$OVERLAYS/PendingResumeWorker.kt" ]]; then
  cp "$OVERLAYS/PendingResumeWorker.kt" "$PLUGIN_DEST_DIR/PendingResumeWorker.kt"
  echo "Installed PendingResumeWorker.kt"
fi

if ! grep -q 'youtubedl-android:library' "$GRADLE"; then
  python3 - "$GRADLE" "$YOUTUBE_DL_ANDROID_VERSION" "$WORK_RUNTIME_KTX_VERSION" <<'PY'
import sys
from pathlib import Path
path = Path(sys.argv[1])
ver = sys.argv[2]
wm = sys.argv[3]
text = path.read_text()
deps = f'''    // youtubedl-android: bundles Python + yt-dlp (pin: {ver})
    implementation("io.github.junkfood02.youtubedl-android:library:{ver}")
    implementation("io.github.junkfood02.youtubedl-android:ffmpeg:{ver}")
    implementation("androidx.work:work-runtime-ktx:{wm}")
'''
needle = "dependencies {"
idx = text.find(needle)
if idx < 0:
    raise SystemExit("Could not find dependencies block")
insert_at = idx + len(needle)
text = text[:insert_at] + "\n" + deps + text[insert_at:]
path.write_text(text)
print(f"Added youtubedl-android {ver} + WorkManager deps to build.gradle.kts")
PY
elif ! grep -q 'work-runtime-ktx' "$GRADLE"; then
  python3 - "$GRADLE" "$WORK_RUNTIME_KTX_VERSION" <<'PY'
import sys
from pathlib import Path
path = Path(sys.argv[1])
wm = sys.argv[2]
text = path.read_text()
deps = f'    implementation("androidx.work:work-runtime-ktx:{wm}")\n'
needle = "dependencies {"
idx = text.find(needle)
if idx < 0:
    raise SystemExit("Could not find dependencies block")
insert_at = idx + len(needle)
text = text[:insert_at] + "\n" + deps + text[insert_at:]
path.write_text(text)
print("Added WorkManager dependency to build.gradle.kts")
PY
else
  echo "youtubedl-android Gradle deps already present (pin $YOUTUBE_DL_ANDROID_VERSION)."
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

echo "Android YtDlp overlay patch complete (youtubedl-android $YOUTUBE_DL_ANDROID_VERSION)."

STALE_BINARIES="$GEN_ANDROID/app/src/main/assets/binaries"
if [[ -d "$STALE_BINARIES" ]]; then
  rm -rf "$STALE_BINARIES"
  echo "Removed stale assets/binaries (Linux ffmpeg sidecars)."
fi
