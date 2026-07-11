import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import { getCapabilities, getAppRuntime } from "@/lib/runtime";
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
        if (caps.localIpc) {
          const backend = await api.getSettings();
          if (!cancelled) {
            setHostSettings(backend);
            updateSettings({
              engine_mode: backend.engine_mode,
              library_path: backend.library_path,
              lan_enabled: backend.lan_enabled,
              lan_port: backend.lan_port,
              lan_token: backend.lan_token,
              phash_threshold: backend.phash_threshold,
            });
          }
        } else if (caps.libraryScanRemote || runtime !== "browser") {
          try {
            const remote = await api.getHostSettings();
            if (!cancelled) setHostSettings(remote);
          } catch {
            if (!cancelled) setHostSettings(settings);
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
    if (caps.localIpc) {
      await api.saveSettings(hostSettings);
      updateSettings({
        library_path: hostSettings.library_path,
        phash_threshold: hostSettings.phash_threshold,
      });
    } else if (caps.libraryScanRemote) {
      await api.saveHostSettings(hostSettings);
    } else {
      updateSettings(hostSettings);
    }
  }, [caps.localIpc, caps.libraryScanRemote, hostSettings, updateSettings]);

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
