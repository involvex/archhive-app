import { useEffect, useRef } from "react";
import { toast } from "react-hot-toast";
import { api } from "@/lib/api/client";
import { shouldUseRemoteApi, isDesktopTauriRuntime } from "@/lib/runtime";
import type { DownloadJob } from "@/lib/types";

const POLL_INTERVAL_MS = 5000;

export function useDownloadNotifications() {
  const prevStatus = useRef<Map<string, DownloadJob["status"]>>(new Map());
  const useRemote = shouldUseRemoteApi();

  useEffect(() => {
    if (isDesktopTauriRuntime() && !useRemote) {
      const unlisten = api
        .subscribeDownloadProgress((job) => {
          const prev = prevStatus.current.get(job.id);
          if (prev !== job.status) {
            prevStatus.current.set(job.id, job.status);
            emitDownloadToast(job);
          }
        })
        .catch(() => {});
      return () => {
        void unlisten.then((fn) => fn?.());
      };
    }

    const poll = async () => {
      try {
        const jobs = await api.listDownloads();
        for (const job of jobs) {
          const prev = prevStatus.current.get(job.id);
          if (prev !== job.status) {
            prevStatus.current.set(job.id, job.status);
            emitDownloadToast(job);
          }
        }
      } catch {
        // Network or auth error — silently skip; next poll retry.
      }
    };

    poll();
    const timer = setInterval(poll, POLL_INTERVAL_MS);
    return () => clearInterval(timer);
  }, [useRemote]);
}

function emitDownloadToast(job: DownloadJob) {
  if (job.status === "completed") {
    toast(`${job.title || job.url} — download complete`, {
      icon: "✓",
      duration: 5000,
    });
  } else if (job.status === "failed") {
    toast(`${job.title || job.url} — download failed`, {
      icon: "✕",
      duration: 8000,
    });
  }
}
