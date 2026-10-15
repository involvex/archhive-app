#!/usr/bin/env bash
# Zipalign + sign a release APK so Android will install it.
#
# `tauri android build` (release, no signingConfig) emits
# app-universal-release-unsigned.apk; Android refuses unsigned APKs
# ("App not installed" / "package appears to be invalid").
#
# Usage: scripts/sign-android-apk.sh <unsigned.apk> <signed-output.apk>
#
# Key (stable across releases, so updates install over each other):
#   ANDROID_KEYSTORE_BASE64   base64 of a .jks / .keystore file
#   ANDROID_KEYSTORE_PASSWORD keystore password
#   ANDROID_KEY_ALIAS         key alias
#   ANDROID_KEY_PASSWORD      key password (defaults to the keystore password)
# Without them a throwaway key is generated: the APK installs, but a later
# build signed with a different key cannot update it (uninstall first).

set -euo pipefail

IN="${1:?usage: sign-android-apk.sh <unsigned.apk> <signed-output.apk>}"
OUT="${2:?usage: sign-android-apk.sh <unsigned.apk> <signed-output.apk>}"
[[ -f "$IN" ]] || { echo "APK not found: $IN" >&2; exit 1; }

# Load repo-root .env (local signing secrets) for vars not already set.
# CI passes the same values as encrypted secrets instead (no .env there).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "$SCRIPT_DIR/../.env" ]]; then
  while IFS='=' read -r name value; do
    [[ "$name" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || continue
    if [[ -z "${!name:-}" ]]; then
      printf -v "$name" '%s' "$value"
      export "$name"
    fi
  done < <(grep '=' "$SCRIPT_DIR/../.env" | grep -v '^[[:space:]]*#')
fi

SDK="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
[[ -n "$SDK" && -d "$SDK/build-tools" ]] || { echo "ANDROID_HOME with build-tools is required" >&2; exit 1; }
BT="$(ls -d "$SDK"/build-tools/*/ 2>/dev/null | sort -V | tail -1 || true)"
[[ -n "$BT" ]] || { echo "No build-tools versions found in $SDK/build-tools" >&2; exit 1; }
ZIPALIGN="$BT/zipalign"
APKSIGNER="$BT/apksigner"
[[ -x "$ZIPALIGN" && -x "$APKSIGNER" ]] || { echo "zipalign/apksigner not found in $BT" >&2; exit 1; }

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

if [[ -n "${ANDROID_KEYSTORE_BASE64:-}" ]]; then
  : "${ANDROID_KEYSTORE_PASSWORD:?ANDROID_KEYSTORE_PASSWORD is required with ANDROID_KEYSTORE_BASE64}"
  : "${ANDROID_KEY_ALIAS:?ANDROID_KEY_ALIAS is required with ANDROID_KEYSTORE_BASE64}"
  KEYSTORE="$WORK/release.jks"
  printf '%s' "$ANDROID_KEYSTORE_BASE64" | base64 -d > "$KEYSTORE"
  STORE_PASS="$ANDROID_KEYSTORE_PASSWORD"
  KEY_PASS="${ANDROID_KEY_PASSWORD:-$ANDROID_KEYSTORE_PASSWORD}"
  ALIAS="$ANDROID_KEY_ALIAS"
  echo "Signing with the configured release key (alias $ALIAS)."
else
  KEYSTORE="$WORK/throwaway.jks"
  STORE_PASS="$(head -c 18 /dev/urandom | base64 | tr -dc 'A-Za-z0-9')"
  KEY_PASS="$STORE_PASS"
  ALIAS="archhive"
  keytool -genkeypair -keystore "$KEYSTORE" -storepass "$STORE_PASS" -keypass "$KEY_PASS" \
    -alias "$ALIAS" -keyalg RSA -keysize 2048 -validity 10000 \
    -dname "CN=ArcHive (unofficial build)" >/dev/null 2>&1
  echo "::warning::No ANDROID_KEYSTORE_BASE64 secret: signed with a throwaway key. The APK installs, but cannot update an install signed with another key."
fi

# -P 16: 16 KB page alignment for uncompressed .so files (Android 15+ devices).
"$ZIPALIGN" -P 16 -f 4 "$IN" "$WORK/aligned.apk"
printf '%s' "$STORE_PASS" > "$WORK/store.pass"
printf '%s' "$KEY_PASS" > "$WORK/key.pass"
mkdir -p "$(dirname "$OUT")"
"$APKSIGNER" sign --ks "$KEYSTORE" --ks-key-alias "$ALIAS" \
  --ks-pass "file:$WORK/store.pass" --key-pass "file:$WORK/key.pass" \
  --out "$OUT" "$WORK/aligned.apk"
"$APKSIGNER" verify --print-certs "$OUT" | head -2
"$ZIPALIGN" -c -P 16 4 "$OUT" >/dev/null && echo "Signed + aligned: $OUT"
