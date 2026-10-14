import { isTauri } from "./tauri";
import { getAppRuntime } from "./runtime";

/** The Android app (in-app browser and share sheet available). */
export function isAndroidApp(): boolean {
  return getAppRuntime() === "mobile-tauri" && /android/i.test(navigator.userAgent);
}

/** Android share sheet for a URL (copy it, or hand it to a browser / app). */
export async function shareUrl(url: string): Promise<void> {
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("android_share", { text: url, path: null, mime: null });
}

/** Android share sheet for a library file (video players and other apps). */
export async function shareFile(path: string, mime = "video/*"): Promise<void> {
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("android_share", { text: null, path, mime });
}

/**
 * Open an external URL in the system browser.
 *
 * Plain `<a target="_blank">` / `window.open` do nothing inside the Tauri
 * webview — external links must go through the opener plugin (`openUrl`),
 * which is registered backend-side (`tauri_plugin_opener::init`) and allowed
 * via `opener:default` in the capabilities. Outside Tauri (plain browser /
 * LAN web UI) fall back to `window.open`.
 */
export async function openExternal(url: string): Promise<void> {
  // Android: the in-app browser (ad blocking, nothing in the system
  // browser's history); the system browser only if it can't open.
  if (isAndroidApp()) {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("inapp_browse", { url, title: null });
      return;
    } catch (e) {
      console.warn("in-app browser failed; using the system browser", e);
    }
  }
  if (isTauri()) {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
    return;
  }
  window.open(url, "_blank", "noopener");
}
