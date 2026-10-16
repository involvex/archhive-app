# Releasing ArcHive

## Version sources

These files stay in sync (use `bun run version:bump <semver>`):

| File                        | Field                            |
| --------------------------- | -------------------------------- |
| `package.json`              | `version`                        |
| `src-tauri/tauri.conf.json` | `version`                        |
| `src-tauri/Cargo.toml`      | `version`                        |
| `src-tauri/Cargo.lock`      | `archhive-app` package `version` |

The UI reads `VITE_APP_VERSION` (from `package.json` at build time) and Tauri `getVersion()` on native. Settings and the desktop sidebar show the current version.

## Cut a release (recommended)

From `D:\repos\archhive-app`:

```powershell
# Bump, commit, tag vX.Y.Z, push — CI builds artifacts
bun run release -- -Version 0.2.0

# Optional: build desktop + APK locally before pushing
bun run release -- -Version 0.2.0 -BuildLocal

# Dry run
bun run release -- -Version 0.2.0 -DryRun
```

Pushing tag `v*` triggers [`.github/workflows/release.yml`](../.github/workflows/release.yml), which:

1. Builds **Windows NSIS installer** → `ArcHive-windows-setup.exe`
2. Builds **Android release APK** (aarch64) → `ArcHive-android.apk`
3. Publishes a **GitHub Release** with both assets

## Manual builds

```powershell
bun run build:desktop      # Windows installer under src-tauri/target/.../bundle/nsis/
bun run build:apk:release  # Release APK under src-tauri/gen/android/.../release/
```

## CI requirements

- **Windows job:** Bun, Rust, sidecar binaries (`setup-binaries.ps1`), Tauri desktop bundle
- **Android job:** Bun, Rust, Android SDK, NDK r27 (`NDK_HOME` required for `tauri android init`), Java 17, `scripts/android-regen.sh` + `tauri icon`

## Android signing

`tauri android build` (release) has no signing config, so Gradle produces
`app-universal-release-unsigned.apk`, which Android refuses to install. The
release workflow runs [`scripts/sign-android-apk.sh`](../scripts/sign-android-apk.sh)
to zipalign (16 KB pages) and sign it. Set these repository secrets so every
release uses the same key — updates then install over the previous version:

| Secret                      | Value                                   |
| --------------------------- | --------------------------------------- |
| `ANDROID_KEYSTORE_BASE64`   | `base64 -w0 release.jks`                |
| `ANDROID_KEYSTORE_PASSWORD` | keystore password                       |
| `ANDROID_KEY_ALIAS`         | key alias                               |
| `ANDROID_KEY_PASSWORD`      | key password (if it differs from above) |

Create a key once with
`keytool -genkeypair -keystore release.jks -alias archhive -keyalg RSA -keysize 4096 -validity 10000`
and keep it safe: losing it means users must uninstall to update.
Store the `.jks` outside the repo (e.g. `~/.archhive/release.jks`); it must
never be committed. Push the secrets with the GitHub CLI:

```powershell
gh secret set ANDROID_KEYSTORE_BASE64 < path\to\release.jks.b64  # base64 of the .jks, single line
gh secret set ANDROID_KEYSTORE_PASSWORD
gh secret set ANDROID_KEY_ALIAS --body archhive
gh secret set ANDROID_KEY_PASSWORD  # same as the keystore password for PKCS12
```

(`gh secret set NAME` without `--body` prompts securely with no shell history.)

Without the secrets the workflow signs with a throwaway key and warns: the APK
installs, but a later build signed with a different key can't update it.
Locally, both signing scripts auto-load a repo-root `.env` (gitignored) so you
don't have to export variables by hand:

```dotenv
ANDROID_KEYSTORE_BASE64=<base64 of release.jks, single line>
ANDROID_KEYSTORE_PASSWORD=<keystore password>
ANDROID_KEY_ALIAS=archhive
ANDROID_KEY_PASSWORD=<key password>
```

Explicit environment variables take precedence over `.env`. Then:

- Windows: `bun run sign:apk` signs the release APK from
  `bun run build:apk:release` into `ArcHive-android.apk` at the repo root
  (`scripts/sign-android-apk.ps1`; pass `-In`, `-Out`, `-Keystore`,
  `-KeyAlias` to override).
- macOS / Linux: `bash scripts/sign-android-apk.sh <unsigned.apk> <out.apk>`
  with the same environment variables. The Windows installer needs no secrets.
