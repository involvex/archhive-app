import { useSettingsStore } from "./stores/settings";
import { isMobileDevice, isTauri } from "./tauri";
import type { EngineMode } from "./types";

export type AppRuntime = "desktop-tauri" | "mobile-tauri" | "browser";

export function getAppRuntime(): AppRuntime {
  if (!isTauri()) return "browser";
  return isMobileDevice() ? "mobile-tauri" : "desktop-tauri";
}

export interface RuntimeCapabilities {
  localIpc: boolean;
  lanServer: boolean;
  libraryPathEditable: boolean;
  libraryScanLocal: boolean;
  libraryScanRemote: boolean;
  showBrowserBanner: boolean;
  lanDiscovery: boolean;
  engineModes: EngineMode[];
}

export function getCapabilities(runtime: AppRuntime = getAppRuntime()): RuntimeCapabilities {
  const { settings } = useSettingsStore.getState();
  const remoteConfigured = Boolean(settings.remote_host?.trim());
  const useRemote =
    runtime === "browser" || runtime === "mobile-tauri" || settings.engine_mode === "remote_lan";

  return {
    localIpc:
      runtime === "desktop-tauri" ||
      (runtime === "mobile-tauri" && settings.engine_mode === "local"),
    lanServer: runtime === "desktop-tauri",
    lanDiscovery: runtime === "mobile-tauri" || runtime === "desktop-tauri",
    libraryPathEditable: runtime === "desktop-tauri",
    libraryScanLocal:
      runtime === "desktop-tauri" ||
      (runtime === "mobile-tauri" && settings.engine_mode === "local"),
    libraryScanRemote: useRemote && remoteConfigured,
    showBrowserBanner: runtime === "browser",
    engineModes:
      runtime === "desktop-tauri"
        ? ["local", "remote_lan", "standalone"]
        : ["local", "remote_lan", "standalone"],
  };
}

export function shouldUseRemoteApi(runtime: AppRuntime = getAppRuntime()): boolean {
  const { settings } = useSettingsStore.getState();
  if (settings.engine_mode === "standalone") return false;
  if (runtime === "browser") return Boolean(settings.remote_host?.trim());
  if (runtime === "mobile-tauri") {
    if (settings.engine_mode === "local") return false;
    return Boolean(settings.remote_host?.trim());
  }
  if (runtime === "desktop-tauri") {
    return settings.engine_mode === "remote_lan" && Boolean(settings.remote_host?.trim());
  }
  return false;
}

export function isDesktopTauriRuntime(): boolean {
  return getAppRuntime() === "desktop-tauri";
}

/**
 * On-device backend available: desktop always; mobile in `local` or
 * `standalone` engine mode (both run the backend in-app, no host needed).
 * Unlike `localIpc` (which also gates desktop-only actions like Explorer),
 * this is purely about "can we serve local files / invoke local commands".
 */
export function hasLocalBackend(runtime: AppRuntime = getAppRuntime()): boolean {
  if (runtime === "desktop-tauri") return true;
  if (runtime !== "mobile-tauri") return false;
  const { settings } = useSettingsStore.getState();
  return settings.engine_mode === "local" || settings.engine_mode === "standalone";
}
