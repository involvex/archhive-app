import { useEffect, useCallback } from "react";
import { api } from "@/lib/api/client";
import { shouldUseRemoteApi, getAppRuntime } from "@/lib/runtime";
import type { DownloadJob } from "@/lib/types";

const POLL_INTERVAL_MS = 5000;

export type ToastFn = (message: string, options?: { icon?: string; duration?: number }) => string;

/**
 * Session-scoped seen-status map. Module-scoped so it survives component
 * remounts (e.g. when the share-target navigates to /downloads) — otherwise
 * every completed/failed job would re-toast on each tab open.
 */
const seenStatus = new Map<string, DownloadJob["status"]>();

export function useDownloadNotifications(toastFn?: ToastFn) {
  const useRemote = shouldUseRemoteApi();
  const runtime = getAppRuntime();

  const emitToast = useCallback(
    (job: DownloadJob) => {
      if (!toastFn) return;
      if (job.status === "completed") {
        toastFn(`${job.title || job.url} — download complete`, {
          icon: "✓",
          duration: 5000,
        });
      } else if (job.status === "failed") {
        toastFn(`${job.title || job.url} — download failed`, {
          icon: "✕",
          duration: 8000,
        });
      }
    },
    [toastFn],
  );

  useEffect(() => {
    // Desktop emits native OS notifications from Rust (#55) — no in-app toasts.
    // Mobile (local or standalone, not remote) uses the polling branch below
    // to surface brief in-app toasts for completed/failed downloads.
    if (runtime === "mobile-tauri" && !useRemote) {
      const unlisten = api
        .subscribeDownloadProgress((job) => {
          const prev = seenStatus.get(job.id);
          if (prev !== job.status) {
            seenStatus.set(job.id, job.status);
            emitToast(job);
          }
        })
        .catch(() => {});
      return () => {
        void unlisten.then((fn) => fn?.());
      };
    }

    // Non-toasting runtimes (desktop, browser, remote LAN): listen once to keep
    // prevStatus in sync so a future tab open doesn't replay stale transitions.
    if (runtime === "mobile-tauri" && useRemote) {
      const poll = async () => {
        try {
          const jobs = await api.listDownloads();
          for (const job of jobs) {
            seenStatus.set(job.id, job.status);
          }
        } catch {
          /* network or auth error — silently retry next poll */
        }
      };
      poll();
      const timer = setInterval(poll, POLL_INTERVAL_MS);
      return () => clearInterval(timer);
    }

    // Desktop / browser / remote: still track via IPC events to keep the store
    // current, but never emit in-app toasts (Rust handles native notifications).
    if (runtime !== "mobile-tauri") {
      const unlisten = api
        .subscribeDownloadProgress((job) => {
          seenStatus.set(job.id, job.status);
        })
        .catch(() => {});
      return () => {
        void unlisten.then((fn) => fn?.());
      };
    }

    return undefined;
  }, [useRemote, runtime, emitToast]);
}
