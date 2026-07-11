import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { AppSettings, EngineMode } from "../types";
import { isMobileDevice } from "../tauri";

interface SettingsState {
  settings: AppSettings;
  setEngineMode: (mode: EngineMode) => void;
  setRemoteHost: (host: string) => void;
  setRemoteToken: (token: string) => void;
  setLanEnabled: (enabled: boolean) => void;
  setLanPort: (port: number) => void;
  setLibraryPath: (path: string) => void;
  updateSettings: (partial: Partial<AppSettings>) => void;
}

const defaultSettings: AppSettings = {
  engine_mode: isMobileDevice() ? "remote_lan" : "local",
  library_path: "",
  naming_template: "{performer}/{title}.{ext}",
  lan_enabled: false,
  lan_port: 8787,
  remote_host: undefined,
};

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      settings: defaultSettings,
      setEngineMode: (mode) => set((s) => ({ settings: { ...s.settings, engine_mode: mode } })),
      setRemoteHost: (host) => set((s) => ({ settings: { ...s.settings, remote_host: host } })),
      setRemoteToken: (token) => set((s) => ({ settings: { ...s.settings, remote_token: token } })),
      setLanEnabled: (enabled) =>
        set((s) => ({ settings: { ...s.settings, lan_enabled: enabled } })),
      setLanPort: (port) => set((s) => ({ settings: { ...s.settings, lan_port: port } })),
      setLibraryPath: (path) => set((s) => ({ settings: { ...s.settings, library_path: path } })),
      updateSettings: (partial) => set((s) => ({ settings: { ...s.settings, ...partial } })),
    }),
    {
      name: "archhive-settings",
      onRehydrateStorage: () => (state) => {
        if (!state || !isMobileDevice()) return;
        if (state.settings.engine_mode !== "remote_lan") {
          state.updateSettings({ engine_mode: "remote_lan" });
        }
      },
    },
  ),
);
