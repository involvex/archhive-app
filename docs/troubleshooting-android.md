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

| Issue                                 | Fix                                            |
| ------------------------------------- | ---------------------------------------------- |
| `Unresolved reference: TauriActivity` | `bun run android:regen`                        |
| Wrong package / old launcher name     | `bun run android:regen`                        |
| Missing LAN cleartext (release)       | `.\scripts\patch-android-lan.ps1` then rebuild |

## WebView / white screen

1. Update **Android System WebView** and **Chrome** from Play Store (required on some Huawei/Honor devices).
2. Clear app data: Settings → Apps → ArcHive → Storage → Clear cache/data.
3. Dev: ensure Vite is reachable when using `tauri android dev` (port 1420 on PC).
4. Remote LAN: use desktop API port **8787**, not Vite 1420.

## Missing launcher icon

If the home-screen icon is generic or missing, ensure [`src-tauri/icons/`](../src-tauri/icons/) exists (run `bun run tauri icon assets/branding/icon-source.png`), then `bun run android:regen` and rebuild the APK.

## Related docs

- [mobile-android.md](mobile-android.md) — setup, LAN, emulator networking
- [cookie-import.md](cookie-import.md) — site cookies for browse/download
