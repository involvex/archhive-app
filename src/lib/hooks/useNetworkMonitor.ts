import { useEffect, useRef } from "react";
import { api } from "@/lib/api/client";
import { useSettingsStore } from "@/lib/stores/settings";

export function useNetworkMonitor() {
  const settings = useSettingsStore((s) => s.settings);
  const lastReportedRef = useRef<{ type: string; metered: boolean } | null>(null);

  useEffect(() => {
    if (!settings.download_on_wifi_only) return;

    const report = async () => {
      try {
        const info = await api.getNetworkInfo();
        const type = info.connection_type;
        const metered = info.metered;

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
  }, [settings.download_on_wifi_only]);
}
