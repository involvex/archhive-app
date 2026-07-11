import type { RuntimeCapabilities } from "../runtime";

export const SETTINGS_TABS = ["engine", "library", "cookies", "duplicates", "lan"] as const;
export type SettingsTab = (typeof SETTINGS_TABS)[number];

export function visibleSettingsTabs(caps: RuntimeCapabilities): SettingsTab[] {
  return SETTINGS_TABS.filter((tab) => {
    if (tab === "lan" && !caps.lanServer) return true;
    return true;
  });
}

export function isTabReadOnly(tab: SettingsTab, caps: RuntimeCapabilities): boolean {
  if (tab === "lan") return !caps.lanServer;
  if (tab === "library" && !caps.libraryPathEditable) return true;
  return false;
}
