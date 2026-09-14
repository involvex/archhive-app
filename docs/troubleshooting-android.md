# Android troubleshooting

## `om.archhive.app: FrameInsert open fail`

### What it means

Logcat lines like:

```
om.archhive.app: FrameInsert open fail: No such file or directory
```

are usually **not** a missing `c` in the package name. The real identifier is **`com.archhive.app`** (see [`src-tauri/tauri.conf.json`](../src-tauri/tauri.conf.json)). Some MIUI/HyperOS log viewers truncate the first character of the process name, so `com.` becomes `om.`.

`FrameInsert` comes from MIUI/Chromium performance tracing (framebuffer insert). It is **low priority** on Xiaomi/Redmi/Poco devices unless the app also shows a white screen, crash, or missing UI.

### When to ignore vs investigate

| Symptom                               | Action                                                      |
| ------------------------------------- | ----------------------------------------------------------- |
| App works; only FrameInsert in logcat | Ignore — document for your own sanity                       |
| White screen after splash             | See WebView section below                                   |
| `invoke is undefined` / empty browse  | Engine → Remote LAN; desktop LAN on port **8787**           |
| `ERR_FILE_NOT_FOUND` / blank WebView  | Run `bun run build`; ensure `dist/` exists before packaging |

### Verify package name

After any identifier change, regenerate the Android project:

```powershell
bun run android:regen
```

Then confirm generated Gradle uses `com.archhive.app`:

```powershell
Select-String -Path src-tauri\gen\android\**\*.gradle* -Pattern "archhive" -Recurse
```

Expected: `applicationId` / namespace references `com.archhive.app`, not `com.scrawler` or truncated variants.

## Stale `gen/android`

| Issue                                 | Fix                                              |
| ------------------------------------- | ------------------------------------------------ |
| `Unresolved reference: TauriActivity` | `bun run android:regen`                          |
| Wrong package / old launcher name     | `bun run android:regen`                          |
| Missing LAN cleartext (release)       | `.\scripts\patch-android-lan.ps1` then rebuild   |
| Missing YtDlpPlugin after regen       | `.\scripts\patch-android-ytdlp.ps1` then rebuild |
| Release crash on load (abort/unwind)  | See **Crash on load** below                      |

## Crash on load (Rust abort / `stop_unwind`)

If the app closes immediately after splash and logcat shows `libarchhive_app_lib.so` + `stop_unwind` / `abort`, a Rust panic happened inside `run()`.

Capture the panic message:

```powershell
adb logcat -c
adb shell am force-stop com.archhive.app
adb shell monkey -p com.archhive.app -c android.intent.category.LAUNCHER 1
adb logcat -d | Select-String -Pattern "unwind|YtDlp|error while running|Failed to register|failed to resolve|RustStdoutStderr|FATAL"
```

Common causes:

| Log snippet                             | Fix                                                                                          |
| --------------------------------------- | -------------------------------------------------------------------------------------------- |
| `Failed to register YtDlpPlugin`        | Run `.\scripts\patch-android-ytdlp.ps1` (ProGuard keeps + plugin class); rebuild release APK |
| `error while running tauri application` | Same as above, or check setup errors (DB / app data dir) in the same logcat window           |
| `no such table: collections` on boot    | Install a build with the collections migration fix (no longer gated on FTS notes)            |
| Empty `gen/android` after regen         | Always run `bun run android:regen` (applies LAN + YtDlp overlays)                            |

Release builds enable R8 minify; keep rules live in [`src-tauri/android-overlays/proguard-ytdlp.pro`](../src-tauri/android-overlays/proguard-ytdlp.pro).

## WebView / white screen

1. Update **Android System WebView** and **Chrome** from Play Store (required on some Huawei/Honor devices).
2. Clear app data: Settings → Apps → ArcHive → Storage → Clear cache/data.
3. Dev: ensure Vite is reachable when using `tauri android dev` (port 1420 on PC).
4. Remote LAN: use desktop API port **8787**, not Vite 1420.

## ffmpeg / ffprobe show “not found” in Settings

Standalone Android does **not** use desktop `setup:binaries:android` / BtbN Linux sidecars. Those are glibc ELF binaries and cannot run on Android/bionic — even if the Tools card said “resolved”.

Media tools come from the **youtubedl-android FFmpeg AAR** (`io.github.junkfood02.youtubedl-android:ffmpeg`), applied by [`scripts/patch-android-ytdlp.ps1`](../scripts/patch-android-ytdlp.ps1). At runtime `FFmpeg.init` unpacks libs and `YtDlpPlugin` runs `libffmpeg.so` / `libffprobe.so` with `LD_LIBRARY_PATH` pointing at the unpacked `packages/ffmpeg/usr/lib`.

| Symptom                                                     | Fix                                                                                                                                                      |
| ----------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Settings → Media tools: ffmpeg/ffprobe “not found”          | Rebuild after `bun run android:regen` (or `.\scripts\patch-android-ytdlp.ps1`); open app once so `FFmpeg.init` can unpack (~35MB), then Refresh versions |
| logcat: `CANNOT LINK … libavdevice.so` / `libc++_shared.so` | Rebuild with current `YtDlpPlugin.kt` overlay (`LD_LIBRARY_PATH` must include ffmpeg + python `usr/lib`)                                                 |
| logcat: `media tools ok=false` / paths missing              | Confirm Gradle has the `ffmpeg` AAR dependency; wipe app data and relaunch                                                                               |
| Still running `setup:binaries:android` for media tools      | Skip it — it no longer downloads Android ffmpeg; APK already includes the AAR                                                                            |

Confirm after install:

```powershell
adb logcat -c
adb shell am force-stop com.archhive.app
adb shell monkey -p com.archhive.app -c android.intent.category.LAUNCHER 1
adb logcat -d | Select-String -Pattern "sidecar|YtDlpPlugin|media tools|ffmpeg version|CANNOT LINK"
```

Expect `media tools ok=true` and a real `ffmpeg version …` line (not `CANNOT LINK`).

## Missing launcher icon

If the home-screen icon is generic or missing, ensure [`src-tauri/icons/`](../src-tauri/icons/) exists (run `bun run tauri icon assets/branding/icon-source.png`), then `bun run android:regen` and rebuild the APK.

## Related docs

- [mobile-android.md](mobile-android.md) — setup, LAN, emulator networking
- [cookie-import.md](cookie-import.md) — site cookies for browse/download
