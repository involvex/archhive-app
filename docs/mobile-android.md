# Android development and Remote LAN

## Prerequisites

- Android SDK + NDK (via Android Studio)
- Rust Android targets: `rustup target add aarch64-linux-android armv7-linux-androideabi`
- Java 17+
- Desktop ArcHive running with LAN server enabled

HTTP client uses **rustls** (not OpenSSL), so no `OPENSSL_DIR` / NDK OpenSSL setup is required for Android builds.

Sidecars (`yt-dlp`, `ffmpeg`) are desktop-only. Android builds use `tauri.android.conf.json` with an empty `externalBin` — downloads run on the desktop LAN host.

## One-time setup

```bash
bun install
bun run tauri android init
```

If you changed `identifier` in `tauri.conf.json` (e.g. `com.scrawler` → `com.archhive.app`), regenerate Android:

```powershell
bun run android:regen
```

On Windows, enable cleartext HTTP for LAN (required for `http://192.168.x.x`):

```powershell
.\scripts\patch-android-lan.ps1
```

Debug builds already allow cleartext; the patch is mainly for release APKs.

## Run on device or emulator

```bash
bun run tauri:android:dev
```

On Windows, use the helper script to auto-start an AVD and avoid the interactive device picker:

```powershell
bun run android:dev
```

Or pass a device id explicitly (from `adb devices`):

```bash
bun run tauri android dev emulator-5554
```

Connect a USB device with USB debugging enabled, or start an Android emulator first.

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

1. Close any extra `tauri dev` windows. Run `bun run android:dev` (auto-starts desktop LAN in open mode).
2. **Desktop:** confirm `http://127.0.0.1:8787/api/health` returns `"auth_required": false`.
3. **Phone:** Settings → Engine → Remote LAN → tap discovered **ArcHive @ 192.168.x.x** → **Test Connection**.
4. **Windows Firewall:** allow inbound **TCP 8787** on the desktop PC.
5. Dashboard shows a green connection chip when health succeeds.

### Verification checklist

| Step      | Expected                                                                 |
| --------- | ------------------------------------------------------------------------ |
| Health    | `auth_required: false` when using `ARCHIVE_AUTO_LAN` / `android:dev`     |
| Discovery | mDNS finds PC at `http://192.168.178.69:8787` (your LAN IP)              |
| Browse    | ThotHub search, Reddit channel, PornHub model (with cookies), Custom URL |
| APK       | `bun run build:apk` uses `--target aarch64` only (~15–25 min vs 1h+)     |

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

| Command                           | Description                                      |
| --------------------------------- | ------------------------------------------------ |
| `bun run android:regen`           | Regenerate `gen/android` after identifier change |
| `bun run android:dev`             | Auto-boot AVD + run dev (Windows)                |
| `bun run tauri:android:dev`       | Build and run on connected device/emulator       |
| `bun run tauri android build`     | Release APK/AAB                                  |
| `bun run build:apk`               | Debug APK, aarch64 only (faster)                 |
| `bun run build:apk:fast`          | Skip lint/format; vite build + aarch64 APK       |
| `.\scripts\patch-android-lan.ps1` | Allow HTTP + mDNS multicast on Android           |
