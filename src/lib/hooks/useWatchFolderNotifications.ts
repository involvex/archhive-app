import { useEffect } from "react";
import { api } from "@/lib/api/client";
import { getAppRuntime } from "@/lib/runtime";
import { shouldNotify } from "@/lib/shouldNotify";
import { useSettingsStore } from "@/lib/stores/settings";
import type { ToastFn } from "@/lib/hooks/useDownloadNotifications";

/** In-app toast when the desktop watcher emits `library:watch-import`. */
export function useWatchFolderNotifications(toastFn?: ToastFn) {
  useEffect(() => {
    if (!toastFn) return;
    if (getAppRuntime() === "browser") return;

    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void api
      .subscribeWatchImport((event) => {
        if (cancelled) return;
        const settings = useSettingsStore.getState().settings;
        if (!shouldNotify(settings, "watch_import")) return;
        toastFn(`Auto-imported "${event.title}"`, { duration: 5000 });
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
