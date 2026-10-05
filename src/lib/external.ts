import { isTauri } from "./tauri";

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
  if (isTauri()) {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
    return;
  }
  window.open(url, "_blank", "noopener");
}
