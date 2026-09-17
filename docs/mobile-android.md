# Android development and Remote LAN

## Prerequisites

- Android SDK + NDK (via Android Studio)
- Rust Android targets: `rustup target add aarch64-linux-android armv7-linux-androideabi`
- Java 17+
- Desktop ArcHive running with LAN server enabled

HTTP client uses **rustls** (not OpenSSL), so no `OPENSSL_DIR` / NDK OpenSSL setup is required for Android builds.

Android standalone builds ship an embedded yt-dlp engine (youtubedl-android Kotlin plugin)
plus Android-native ffmpeg/ffprobe from the same AAR (`FFmpeg.init` unpacks
`libffmpeg.zip.so` at runtime). You do **not** need `setup:binaries:android` for
media tools — they are included with the APK via Gradle dependencies applied by
`scripts/apply-android-patches.ps1` (LAN + yt-dlp) / `bun run android:regen`.
APK scripts (`build:apk`, `build:apk:fast`, `build:apk:release`) re-apply
patches and run `scripts/validate-android-build.ps1` before packaging.

**youtubedl-android version pin:** `0.18.1` (library + ffmpeg AARs) is set once at the top of
[`scripts/patch-android-ytdlp.ps1`](../scripts/patch-android-ytdlp.ps1) /
[`scripts/patch-android-ytdlp.sh`](../scripts/patch-android-ytdlp.sh). To upgrade: bump the
constant, run `bun run android:patches`, smoke `YtDlpPlugin` execute + `FFmpeg.init` on device.

Remote LAN mode offloads downloads to the desktop host and needs no on-device engines.

## One-time setup

```bash
bun install
bun run tauri android init
```

If you changed `identifier` in `tauri.conf.json` (e.g. `com.scrawler` → `com.archhive.app`), regenerate Android:

```powershell
bun run android:regen
```

`android:regen` also applies [`scripts/apply-android-patches.ps1`](../scripts/apply-android-patches.ps1), which runs:

- [`scripts/patch-android-lan.ps1`](../scripts/patch-android-lan.ps1) — cleartext HTTP + media permissions + PiP + FG service
- [`scripts/patch-android-ytdlp.ps1`](../scripts/patch-android-ytdlp.ps1) — restores `YtDlpPlugin.kt`, `DownloadForegroundService.kt`, `PendingResumeWorker.kt`, youtubedl-android **0.18.1** Gradle deps, WorkManager, and ProGuard keep rules from [`src-tauri/android-overlays/`](../src-tauri/android-overlays/)

Then it runs [`scripts/validate-android-build.ps1`](../scripts/validate-android-build.ps1) (`-Strict`) so a broken overlay fails the regen.

**Do not skip the YtDlp overlay.** Without it, release R8 minify or a fresh `tauri android init` can leave `register_android_plugin("YtDlpPlugin")` failing at boot.

SDK / `minSdk` / permissions live in the patched Gradle + Manifest after overlays — [`src-tauri/tauri.android.conf.json`](../src-tauri/tauri.android.conf.json) is only a merge overlay for `lan-ui` resources.

On Windows, re-apply + validate without a full regen:

```powershell
bun run android:patches
bun run android:validate
# or:
.\scripts\apply-android-patches.ps1
.\scripts\validate-android-build.ps1
```

Debug builds already allow cleartext; the LAN patch is mainly for release APKs.

## Run on device or emulator

**Use one of these (they share the same helper now):**

```powershell
bun run android:dev
bun run tauri:android:dev
```

That script:

1. Boots an AVD if needed
2. Sets `TAURI_DEV_HOST` / `--host` to your PC LAN IP (fixes Android “Website not available” — `localhost` on the phone is not your PC)
3. Uses a separate `CARGO_TARGET_DIR` (`*-android`) so a desktop `tauri dev` cannot steal the cargo file lock
4. Does **not** start desktop ArcHive by default (standalone Local engine)

For Remote LAN testing (desktop API on :8787):

```powershell
# Option A — dedicated flag (starts desktop in another window, then Android)
bun run android:dev:lan

# Option B — two terminals (clearest when debugging locks)
# Terminal 1:
$env:ARCHIVE_AUTO_LAN='1'; bun run tauri dev
# Terminal 2 (after http://127.0.0.1:8787/api/health is up):
bun run android:dev
```

Or pass a device id explicitly (from `adb devices`):

```powershell
bun run android:dev -- -Device emulator-5554
```

## Physical device over Wi‑Fi (ADB wireless)

Phone and PC must be on the same LAN (e.g. PC `192.168.178.69`, phone `192.168.178.90`).

```powershell
adb devices                    # confirm device listed
bun run tauri android dev <device-id>
```

**Two different ports — don't mix them up:**

| Port   | Purpose                                                                         |
| ------ | ------------------------------------------------------------------------------- |
| `1420` | Vite dev UI only (`http://192.168.178.69:1420`) — loaded by `tauri android dev` |
| `8787` | **Remote LAN API** — set this in app Settings → Engine → Remote LAN             |

Opening `http://<pc-ip>:1420` in **Chrome** is not the Android app — it is browser-only dev UI. Use the installed APK from `bun run android:dev`, or configure Remote LAN on port **8787** in browser mode.

On a **physical phone**, Remote LAN host must be your **PC LAN IP**:

```
http://192.168.178.69:8787
```

`10.0.2.2` is **emulator-only** and will not work on a real device.

Allow both ports through Windows Firewall on the desktop.

## Remote LAN test flow

1. Prefer `bun run android:dev` (sets LAN IP for Vite). Do **not** run bare `tauri android dev` without `--host` — the WebView will try `localhost` on the phone and show “Website not available”.
2. Close extra desktop `tauri dev` windows unless you need Remote LAN. Dual cargo watchers need separate target dirs (the helper sets `*-android` automatically).
3. Run `bun run android:dev` (auto-starts AVD; add `android:dev:lan` only when you need desktop :8787).
4. **Desktop:** confirm `http://127.0.0.1:8787/api/health` returns `"auth_required": false` when using LAN.
5. **Phone:** Settings → Engine → Remote LAN → tap discovered **ArcHive @ 192.168.x.x** → **Test Connection**.
6. **Windows Firewall:** allow inbound **TCP 8787** (and **1420** for Vite hot reload) on the desktop PC.
7. Dashboard shows a green connection chip when health succeeds.

### Verification checklist

| Step      | Expected                                                                 |
| --------- | ------------------------------------------------------------------------ |
| Health    | `auth_required: false` when using `ARCHIVE_AUTO_LAN` / `android:dev`     |
| Discovery | mDNS finds PC at `http://192.168.178.69:8787` (your LAN IP)              |
| Browse    | ThotHub search, Reddit channel, PornHub model (with cookies), Custom URL |
| APK       | `bun run build:apk` uses `--target aarch64` only (~15–25 min vs 1h+)     |

## LAN web UI (browser)

When the desktop LAN server is enabled, the built React app is served at **`http://<pc-ip>:8787/`** on your local network (not port 1420).

| URL                              | Purpose                                                    |
| -------------------------------- | ---------------------------------------------------------- |
| `http://<pc-ip>:8787/`           | Full ArcHive UI (browse, library, downloads)               |
| `http://<pc-ip>:8787/files`      | Folder browser under your library path (like `bunx serve`) |
| `http://<pc-ip>:8787/?token=...` | First-time browser access when LAN auth is enabled         |

Desktop: Settings → LAN → **Copy web link** copies the URL with token.

Video playback uses `GET /api/scenes/{id}/media` and `GET /api/files/stream` with HTTP Range support (seek in browser).

## Remote LAN test flow (manual)

1. **Desktop:** Settings → LAN → enable server (default port `8787`). Copy API token.
2. **Desktop:** `bun run build` so LAN can serve `dist/` (optional but recommended).
3. **Find desktop IP:** `ipconfig` (Windows). Example: `192.168.178.69`.
4. **Phone/emulator:** Settings → Engine → **Remote LAN**
   - Tap a host under **LAN discovery** (mDNS), or enter manually:
   - **Emulator:** `http://10.0.2.2:8787` (listed automatically)
   - **Physical device:** pick discovered desktop host, e.g. `http://192.168.178.69:8787`
   - Token: optional when desktop runs with `ARCHIVE_AUTO_LAN` (e.g. `bun run android:dev`)
5. Tap **Test Connection** — should show desktop app version.
6. Browse sites and queue downloads; jobs run on the desktop host.

## Emulator networking notes

- Android emulator accessing host machine: use `http://10.0.2.2:8787` instead of LAN IP.
- Physical device must be on the same Wi‑Fi as the desktop.

## Troubleshooting

| Issue                                  | Fix                                                                                      |
| -------------------------------------- | ---------------------------------------------------------------------------------------- |
| `FrameInsert open fail` in logcat      | Usually MIUI noise — see [troubleshooting-android.md](troubleshooting-android.md)        |
| `om.archhive.app` in logs              | Truncated `com.archhive.app` — verify with `android:regen` if UI is broken               |
| -------------------------------------- | ---------------------------------------------------------------------------------------- |
| `yt-dlp-aarch64-linux-android` missing | Sidecars are desktop-only; run `bun run android:regen`                                   |
| `Unresolved reference: TauriActivity`  | Stale `gen/android` after identifier change; run `bun run android:regen`                 |
| App runs but `invoke` is undefined     | Set Engine → **Remote LAN**; ensure desktop LAN is on. Rebuild after capability changes. |
| Read-only filesystem on Android        | Mobile uses app data dir for DB; downloads go via Remote LAN desktop host                |
| `ERR_CLEARTEXT_NOT_PERMITTED`          | Run `scripts/patch-android-lan.ps1` and rebuild                                          |
| Connection refused                     | Firewall: allow TCP **8787** on PC (API). `:1420` is dev UI only.                        |
| Used `:1420` in Remote LAN             | Remote LAN API is port **8787**, not the Vite dev port 1420                              |
| 401 Unauthorized                       | Verify API token matches desktop LAN token                                               |
| Empty browse results                   | Ensure desktop app is running and cookies are configured                                 |

## Scripts

| Command                                | Description                                            |
| -------------------------------------- | ------------------------------------------------------ |
| `bun run android:regen`                | Regenerate `gen/android` + apply patches + validate    |
| `bun run android:patches`              | Re-apply LAN + yt-dlp overlays without full regen      |
| `bun run android:validate`             | Preflight check for patched `gen/android`              |
| `bun run android:dev`                  | AVD + Android dev (LAN IP host, separate cargo target) |
| `bun run android:dev:lan`              | Same + auto-start desktop LAN on :8787                 |
| `bun run tauri:android:dev`            | Alias of `android:dev`                                 |
| `bun run tauri android build`          | Release APK/AAB                                        |
| `bun run build:apk`                    | Debug APK, aarch64 only (patches + validate)           |
| `bun run build:apk:fast`               | Skip lint/format; vite build + aarch64 APK             |
| `.\scripts\apply-android-patches.ps1`  | Atomic LAN + yt-dlp patch apply                        |
| `.\scripts\validate-android-build.ps1` | Fail if overlays / cleartext / YtDlpPlugin incomplete  |
| `.\scripts\patch-android-lan.ps1`      | Allow HTTP + mDNS multicast on Android                 |
