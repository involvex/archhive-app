import { useEffect, useRef } from "react";
import { api } from "@/lib/api/client";
import { useSettingsStore } from "@/lib/stores/settings";

type NetConnection = {
  type?: string;
  effectiveType?: string;
  saveData?: boolean;
  addEventListener?: (type: string, listener: () => void) => void;
  removeEventListener?: (type: string, listener: () => void) => void;
};

function readClientNetwork(): { type: string; metered: boolean } {
  if (typeof navigator === "undefined" || !navigator.onLine) {
    return { type: "none", metered: true };
  }

  const nav = navigator as Navigator & {
    connection?: NetConnection;
    mozConnection?: NetConnection;
  };
  const conn = nav.connection ?? nav.mozConnection;
  if (!conn) {
    // Assume Wi-Fi / unmetered when Network Information API is missing (desktop).
    return { type: "wifi", metered: false };
  }

  const rawType = (conn.type || conn.effectiveType || "unknown").toLowerCase();
  let type = "unknown";
  if (rawType === "wifi" || rawType === "ethernet") {
    type = rawType;
  } else if (
    rawType === "cellular" ||
    rawType === "wimax" ||
    rawType.startsWith("2g") ||
    rawType.startsWith("3g") ||
    rawType.startsWith("4g") ||
    rawType.startsWith("5g") ||
    rawType === "slow-2g"
  ) {
    type = "cellular";
  } else if (rawType === "none") {
    type = "none";
  }

  const metered =
    Boolean(conn.saveData) || type === "cellular" || type === "none" || type === "unknown";

  return { type, metered };
}

export function useNetworkMonitor() {
  const settings = useSettingsStore((s) => s.settings);
  const lastReportedRef = useRef<{ type: string; metered: boolean } | null>(null);

  const needsMonitor =
    Boolean(settings.download_on_wifi_only) ||
    Boolean(settings.pause_on_battery_saver) ||
    (settings.data_saver !== undefined && settings.data_saver !== "off");

  useEffect(() => {
    if (!needsMonitor) return;

    const report = async () => {
      try {
        const { type, metered } = readClientNetwork();

        if (
          lastReportedRef.current?.type === type &&
          lastReportedRef.current?.metered === metered
        ) {
          return;
        }
        lastReportedRef.current = { type, metered };

        await api.updateNetworkState(type, metered);
      } catch (e) {
        console.warn("[network] failed to report network state:", e);
      }
    };

    void report();

    const onChange = () => {
      lastReportedRef.current = null;
      void report();
    };

    window.addEventListener("online", onChange);
    window.addEventListener("offline", onChange);

    const nav = navigator as Navigator & {
      connection?: NetConnection;
      mozConnection?: NetConnection;
    };
    const conn = nav.connection ?? nav.mozConnection;
    conn?.addEventListener?.("change", onChange);

    return () => {
      window.removeEventListener("online", onChange);
      window.removeEventListener("offline", onChange);
      conn?.removeEventListener?.("change", onChange);
    };
  }, [needsMonitor]);
}
