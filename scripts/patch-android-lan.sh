#!/usr/bin/env bash
# Patches generated Android project: cleartext HTTP + media storage permissions.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GRADLE="$ROOT/src-tauri/gen/android/app/build.gradle.kts"
MANIFEST="$ROOT/src-tauri/gen/android/app/src/main/AndroidManifest.xml"

if [[ ! -f "$GRADLE" ]]; then
  echo "Android project not found. Run: bun run tauri android init" >&2
  exit 1
fi

if grep -q 'manifestPlaceholders\["usesCleartextTraffic"\] = "true"' "$GRADLE"; then
  echo "Android LAN cleartext patch already applied."
else
  sed -i 's/manifestPlaceholders\["usesCleartextTraffic"\] = "false"/manifestPlaceholders["usesCleartextTraffic"] = "true"/' "$GRADLE"
  echo "Patched $GRADLE for cleartext HTTP (Remote LAN)."
fi

if [[ ! -f "$MANIFEST" ]]; then
  exit 0
fi

if grep -q 'READ_MEDIA_VIDEO' "$MANIFEST"; then
  if grep -q 'CHANGE_WIFI_MULTICAST_STATE' "$MANIFEST"; then
    echo "Android storage permissions already present."
  else
    sed -i '/<uses-permission android:name="android.permission.INTERNET" \/>/a\
    <uses-permission android:name="android.permission.CHANGE_WIFI_MULTICAST_STATE" />' "$MANIFEST"
    echo "Patched $MANIFEST with CHANGE_WIFI_MULTICAST_STATE for mDNS."
  fi
else
  sed -i '/<uses-permission android:name="android.permission.INTERNET" \/>/a\
    <uses-permission android:name="android.permission.CHANGE_WIFI_MULTICAST_STATE" />\
    <uses-permission android:name="android.permission.READ_MEDIA_VIDEO" />\
    <uses-permission android:name="android.permission.READ_MEDIA_IMAGES" />\
    <uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE" android:maxSdkVersion="32" />' "$MANIFEST"
  echo "Patched $MANIFEST with mDNS and media storage permissions."
fi

# Enable Activity PiP so HTML video.requestPictureInPicture can float.
if grep -q 'supportsPictureInPicture' "$MANIFEST"; then
  echo "Android Picture-in-Picture already enabled."
else
  if grep -q 'android:configChanges="' "$MANIFEST"; then
    sed -i 's/android:configChanges="[^"]*"/& android:supportsPictureInPicture="true" android:resizeableActivity="true"/' "$MANIFEST"
  else
    sed -i 's/<activity /<activity android:supportsPictureInPicture="true" android:resizeableActivity="true" /' "$MANIFEST"
  fi
  echo "Patched $MANIFEST for Picture-in-Picture."
fi
