# Android development and Remote LAN

## Prerequisites

- Android SDK + NDK (via Android Studio)
- Rust Android targets: `rustup target add aarch64-linux-android armv7-linux-androideabi`
- Java 17+
- Desktop Scrawler running with LAN server enabled

## One-time setup

```bash
bun install
bun run tauri android init
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

## Remote LAN test flow

1. **Desktop:** Settings → LAN → enable server (default port `8787`). Copy API token.
2. **Desktop:** `bun run build` so LAN can serve `dist/` (optional but recommended).
3. **Find desktop IP:** `ipconfig` (Windows) or `ip addr` (Linux). Example: `192.168.1.42`.
4. **Phone/emulator:** Settings → Engine → **Remote LAN**
   - Host: `http://192.168.1.42:8787`
   - Token: paste from desktop
5. Tap **Test Connection** — should show desktop app version.
6. Browse sites and queue downloads; jobs run on the desktop host.

## Emulator networking notes

- Android emulator accessing host machine: use `http://10.0.2.2:8787` instead of LAN IP.
- Physical device must be on the same Wi‑Fi as the desktop.

## Troubleshooting

| Issue                         | Fix                                                      |
| ----------------------------- | -------------------------------------------------------- |
| `ERR_CLEARTEXT_NOT_PERMITTED` | Run `scripts/patch-android-lan.ps1` and rebuild          |
| Connection refused            | Check firewall allows inbound TCP on LAN port            |
| 401 Unauthorized              | Verify API token matches desktop LAN token               |
| Empty browse results          | Ensure desktop app is running and cookies are configured |

## Scripts

| Command                           | Description                                |
| --------------------------------- | ------------------------------------------ |
| `bun run android:dev`             | Auto-boot AVD + run dev (Windows)          |
| `bun run tauri:android:dev`       | Build and run on connected device/emulator |
| `bun run tauri android build`     | Release APK/AAB                            |
| `.\scripts\patch-android-lan.ps1` | Allow HTTP to LAN host                     |
