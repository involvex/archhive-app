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

No secrets required for unsigned APK + NSIS upload. Add code signing later if needed.
