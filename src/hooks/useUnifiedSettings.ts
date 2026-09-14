import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import { getCapabilities, getAppRuntime, hasLocalBackend } from "@/lib/runtime";
import { useSettingsStore } from "@/lib/stores/settings";
import type { AppSettings } from "@/lib/types";

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
        // hasLocalBackend (not localIpc) so mobile standalone — which runs
        // the backend in-app — also loads/saves real backend settings.
        if (hasLocalBackend(runtime)) {
          const backend = await api.getSettings();
          if (!cancelled) {
            setHostSettings(backend);
            const priorHost = useSettingsStore.getState().settings.remote_host?.trim();
            const priorToken = useSettingsStore.getState().settings.remote_token?.trim();
            const remoteHost = backend.remote_host?.trim() || priorHost || undefined;
            const remoteToken = backend.remote_token?.trim() || priorToken || undefined;
            updateSettings({
              engine_mode: backend.engine_mode,
              library_path: backend.library_path,
              lan_enabled: backend.lan_enabled,
              lan_port: backend.lan_port,
              lan_token: backend.lan_token,
              remote_host: remoteHost,
              remote_token: remoteToken,
              phash_threshold: backend.phash_threshold,
              close_to_tray: backend.close_to_tray,
              minimize_to_tray: backend.minimize_to_tray,
              tray_hotkey: backend.tray_hotkey,
            });
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
    if (hasLocalBackend(runtime)) {
      await api.saveSettings(hostSettings);
      updateSettings({
        engine_mode: hostSettings.engine_mode,
        library_path: hostSettings.library_path,
        remote_host: hostSettings.remote_host,
        remote_token: hostSettings.remote_token,
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
