import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import { getCapabilities } from "@/lib/runtime";
import { useSettingsStore } from "@/lib/stores/settings";
import type { HealthResponse } from "@/lib/types";

export type LanConnectionState = "idle" | "checking" | "connected" | "error";

export interface LanConnectionStatus {
  state: LanConnectionState;
  health: HealthResponse | null;
  message: string;
  refresh: () => Promise<void>;
}

export function useLanConnection(pollMs = 15000): LanConnectionStatus {
  const remoteHost = useSettingsStore((s) => s.settings.remote_host);
  const engineMode = useSettingsStore((s) => s.settings.engine_mode);
  const caps = getCapabilities();
  const needsRemote =
    caps.showBrowserBanner || (engineMode === "remote_lan" && Boolean(remoteHost?.trim()));

  const [state, setState] = useState<LanConnectionState>("idle");
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [message, setMessage] = useState("");

  const refresh = useCallback(async () => {
    if (!needsRemote) {
      setState("connected");
      setHealth(null);
      setMessage("Local desktop engine");
      return;
    }
    if (!remoteHost?.trim()) {
      setState("error");
      setHealth(null);
      setMessage("No remote host — open Settings and pick a discovered LAN host.");
      return;
    }
    setState("checking");
    try {
      const h = await api.health();
      setHealth(h);
      setState("connected");
      const authNote =
        h.auth_required === true ? " (token required)" : h.auth_required === false ? " (open)" : "";
      setMessage(`Connected to ArcHive v${h.version}${authNote}`);
    } catch (e) {
      setHealth(null);
      setState("error");
      setMessage(e instanceof Error ? e.message : "Connection failed");
    }
  }, [needsRemote, remoteHost]);

  useEffect(() => {
    void queueMicrotask(() => void refresh());
    if (!needsRemote || !remoteHost?.trim()) return;
    const id = window.setInterval(() => void refresh(), pollMs);
    return () => window.clearInterval(id);
  }, [refresh, needsRemote, remoteHost, pollMs]);

  return { state, health, message, refresh };
}
