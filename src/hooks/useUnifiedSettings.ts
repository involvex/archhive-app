import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import { getCapabilities, getAppRuntime, hasLocalBackend } from "@/lib/runtime";
import { useSettingsStore } from "@/lib/stores/settings";
import type { AppSettings } from "@/lib/types";

function applyDeviceSettingsToStore(backend: AppSettings) {
  const priorHost = useSettingsStore.getState().settings.remote_host?.trim();
  const priorToken = useSettingsStore.getState().settings.remote_token?.trim();
  const remoteHost = backend.remote_host?.trim() || priorHost || undefined;
  const remoteToken = backend.remote_token?.trim() || priorToken || undefined;
  useSettingsStore.getState().updateSettings({
    engine_mode: backend.engine_mode,
    library_path: backend.library_path,
    lan_enabled: backend.lan_enabled,
    lan_port: backend.lan_port,
    lan_token: backend.lan_token,
    lan_auth_enabled: backend.lan_auth_enabled,
    remote_host: remoteHost,
    remote_token: remoteToken,
    phash_threshold: backend.phash_threshold,
    close_to_tray: backend.close_to_tray,
    minimize_to_tray: backend.minimize_to_tray,
    tray_hotkey: backend.tray_hotkey,
  });
  return { remoteHost, remoteToken, priorHost, priorToken };
}

export function useUnifiedSettings() {
  const runtime = getAppRuntime();
  const caps = getCapabilities(runtime);
  const { settings, updateSettings } = useSettingsStore();
  const [hostSettings, setHostSettings] = useState<AppSettings | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  useEffect(() => {
    let cancelled = false;
    async function load() {
      setLoading(true);
      setError("");
      try {
        // Mobile: always load device SQLite for engine/remote pairing first.
        // Remote host settings are a separate mirror for library ops only.
        if (runtime === "mobile-tauri") {
          const device = await api.getDeviceSettings();
          if (cancelled) return;
          const { remoteHost, remoteToken, priorHost, priorToken } =
            applyDeviceSettingsToStore(device);

          if (priorHost && !device.remote_host?.trim()) {
            const merged = {
              ...device,
              remote_host: priorHost,
              remote_token: priorToken || device.remote_token,
            };
            void api.saveDeviceSettings(merged).catch(() => {});
            if (!cancelled) setHostSettings(merged);
          } else if (
            device.engine_mode === "remote_lan" &&
            (remoteHost || device.remote_host)?.trim()
          ) {
            try {
              const remote = await api.getHostSettings();
              if (!cancelled) setHostSettings(remote);
            } catch {
              if (!cancelled) setHostSettings(device);
            }
          } else {
            if (!cancelled) setHostSettings(device);
          }

          // Keep remote_* in store from device (or prior) after host mirror load.
          if (remoteHost) {
            updateSettings({
              remote_host: remoteHost,
              remote_token: remoteToken,
            });
          }
        } else if (hasLocalBackend(runtime)) {
          const backend = await api.getSettings();
          if (!cancelled) {
            setHostSettings(backend);
            const { priorHost, priorToken } = applyDeviceSettingsToStore(backend);
            if (priorHost && !backend.remote_host?.trim()) {
              const merged = {
                ...backend,
                remote_host: priorHost,
                remote_token: priorToken || backend.remote_token,
              };
              void api.saveSettings(merged).catch(() => {});
              setHostSettings(merged);
            }
          }
        } else if (runtime !== "browser") {
          if (!settings.remote_host?.trim()) {
            if (!cancelled) setHostSettings(settings);
          } else {
            try {
              const remote = await api.getHostSettings();
              if (!cancelled) setHostSettings(remote);
            } catch {
              if (!cancelled) setHostSettings(settings);
            }
          }
        } else {
          if (!cancelled) setHostSettings(settings);
        }
      } catch (e) {
        if (!cancelled) {
          setError(e instanceof Error ? e.message : "Failed to load settings");
          setHostSettings(settings);
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    void load();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- mount + runtime
  }, [runtime]);

  const saveHostSettings = useCallback(async () => {
    if (!hostSettings) return;
    if (runtime === "mobile-tauri") {
      const mode = useSettingsStore.getState().settings.engine_mode;
      if (mode === "local" || mode === "standalone") {
        await api.saveDeviceSettings(hostSettings);
        updateSettings({
          engine_mode: hostSettings.engine_mode,
          library_path: hostSettings.library_path,
          remote_host: hostSettings.remote_host,
          remote_token: hostSettings.remote_token,
          lan_auth_enabled: hostSettings.lan_auth_enabled,
          phash_threshold: hostSettings.phash_threshold,
        });
      } else if (caps.libraryScanRemote) {
        await api.saveHostSettings(hostSettings);
      } else {
        await api.saveDeviceSettings(hostSettings);
        updateSettings(hostSettings);
      }
    } else if (hasLocalBackend(runtime)) {
      await api.saveSettings(hostSettings);
      updateSettings({
        engine_mode: hostSettings.engine_mode,
        library_path: hostSettings.library_path,
        remote_host: hostSettings.remote_host,
        remote_token: hostSettings.remote_token,
        lan_auth_enabled: hostSettings.lan_auth_enabled,
        phash_threshold: hostSettings.phash_threshold,
      });
    } else if (caps.libraryScanRemote) {
      await api.saveHostSettings(hostSettings);
    } else {
      updateSettings(hostSettings);
    }
  }, [caps.libraryScanRemote, hostSettings, runtime, updateSettings]);

  const patchHostSettings = useCallback((partial: Partial<AppSettings>) => {
    setHostSettings((prev) => (prev ? { ...prev, ...partial } : prev));
  }, []);

  return {
    runtime,
    caps,
    settings,
    updateSettings,
    hostSettings,
    setHostSettings,
    patchHostSettings,
    saveHostSettings,
    loading,
    error,
  };
}
