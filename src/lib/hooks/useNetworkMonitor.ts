import { useEffect, useRef } from "react";
import { api } from "@/lib/api/client";
import { useSettingsStore } from "@/lib/stores/settings";

interface NetworkInfo extends EventTarget {
  type: string;
  effectiveType: string;
  downlink: number;
  rtt: number;
  saveData: boolean;
  onchange: ((this: NetworkInfo, ev: Event) => void) | null;
}

declare global {
  interface Navigator {
    connection?: NetworkInfo;
    mozConnection?: NetworkInfo;
    webkitConnection?: NetworkInfo;
  }
}

function getConnection(): NetworkInfo | undefined {
  return navigator.connection || navigator.mozConnection || navigator.webkitConnection;
}

function getConnectionType(conn: NetworkInfo): string {
  return conn.type || conn.effectiveType || "unknown";
}

export function useNetworkMonitor() {
  const settings = useSettingsStore((s) => s.settings);
  const lastReportedRef = useRef<{ type: string; metered: boolean } | null>(null);

  useEffect(() => {
    if (!settings.download_on_wifi_only) return;

    const conn = getConnection();
    if (!conn) return;

    const report = async () => {
      const type = getConnectionType(conn);
      const metered = conn.saveData;

      if (lastReportedRef.current?.type === type && lastReportedRef.current?.metered === metered) {
        return;
      }
      lastReportedRef.current = { type, metered };

      try {
        await api.updateNetworkState(type, metered);
      } catch (e) {
        console.warn("[network] failed to report network state:", e);
      }
    };

    report();
    conn.addEventListener("change", report);
    return () => conn.removeEventListener("change", report);
  }, [settings.download_on_wifi_only]);
}
