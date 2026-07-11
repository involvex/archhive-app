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

1. **Desktop:** Settings → LAN → enable server (default port `8787`). Copy API token.
2. **Desktop:** `bun run build` so LAN can serve `dist/` (optional but recommended).
3. **Find desktop IP:** `ipconfig` (Windows). Example: `192.168.178.69`.
4. **Phone/emulator:** Settings → Engine → **Remote LAN**
   - **Emulator:** `http://10.0.2.2:8787`
   - **Physical device:** `http://192.168.178.69:8787` (your PC IP, port **8787** not 1420)
   - Token: paste from desktop
5. Tap **Test Connection** — should show desktop app version.
6. Browse sites and queue downloads; jobs run on the desktop host.

## Emulator networking notes

- Android emulator accessing host machine: use `http://10.0.2.2:8787` instead of LAN IP.
- Physical device must be on the same Wi‑Fi as the desktop.

## Troubleshooting

| Issue                                  | Fix                                                                                      |
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
| `.\scripts\patch-android-lan.ps1` | Allow HTTP to LAN host                           |
