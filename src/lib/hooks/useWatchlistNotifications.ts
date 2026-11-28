import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { getAppRuntime } from "@/lib/runtime";
import { shouldNotify } from "@/lib/shouldNotify";
import { useSettingsStore } from "@/lib/stores/settings";
import type { WatchlistPollResult } from "@/lib/types";
import type { ToastFn } from "@/lib/hooks/useDownloadNotifications";

/** In-app toast when the Rust watchlist poller emits `watchlist:matches`. */
export function useWatchlistNotifications(toastFn?: ToastFn) {
  useEffect(() => {
    if (!toastFn) return;
    if (getAppRuntime() === "browser") return;

    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void listen<WatchlistPollResult>("watchlist:matches", (event) => {
      if (cancelled) return;
      const settings = useSettingsStore.getState().settings;
      if (!shouldNotify(settings, "saved_search_matches")) return;
      const r = event.payload;
      const matched = r.matched ?? 0;
      if (matched <= 0 && r.queued <= 0) return;
      const parts = [`${matched} new match${matched === 1 ? "" : "es"}`];
      if (r.queued > 0) parts.push(`${r.queued} queued`);
      toastFn(`Watchlist: ${parts.join(", ")}`, { icon: "★", duration: 6000 });
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {});

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [toastFn]);
}
