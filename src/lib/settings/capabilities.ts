import type { RuntimeCapabilities } from "../runtime";

export const SETTINGS_TABS = [
  "engine",
  "library",
  "downloads",
  "cookies",
  "metadata",
  "duplicates",
  "lan",
  "desktop",
  "browse",
  "diagnostics",
] as const;
export type SettingsTab = (typeof SETTINGS_TABS)[number];

export function visibleSettingsTabs(caps: RuntimeCapabilities): SettingsTab[] {
  return SETTINGS_TABS.filter((tab) => {
    // Tray / hotkey / window options only exist in the desktop app — not on
    // phones, even though they also have a local backend.
    if (tab === "desktop" && !caps.desktopShell) return false;
    if (tab === "lan" && !caps.lanServer) return true;
    return true;
  });
}

export function isTabReadOnly(tab: SettingsTab, caps: RuntimeCapabilities): boolean {
  if (tab === "lan") return !caps.lanServer;
  return false;
}
