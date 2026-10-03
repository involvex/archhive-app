import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { DownloadJob } from "@/lib/types";

export interface UseDownloadsResult {
  downloads: DownloadJob[];
  /** Jobs in `active` or `pending` state — what the badge/strip renders. */
  active: DownloadJob[];
  /** Explicit re-read from the backend (pull-to-refresh). */
  refresh: () => Promise<void>;
}

/**
 * Download queue state plus a live progress subscription.
 *
 * Shared by /browse, /feed and / so the queue strip and badge cannot drift.
 * The initial load is seeded once, then Tauri events keep the list current —
 * matching the existing behaviour on each page it replaces.
 */
export function useDownloads(): UseDownloadsResult {
  const [downloads, setDownloads] = useState<DownloadJob[]>([]);

  useEffect(() => {
    let cancelled = false;
    void api
      .listDownloads()
      .then((all) => {
        if (!cancelled) setDownloads(all);
      })
      .catch(() => {
        /* degrade silently — the queue strip just stays empty */
      });

    let unsub: (() => void) | undefined;
    void api
      .subscribeDownloadProgress((job) => {
        setDownloads((prev) => {
          const idx = prev.findIndex((j) => j.id === job.id);
          if (idx === -1) return [job, ...prev];
          const next = [...prev];
          next[idx] = job;
          return next;
        });
      })
      .then((fn) => {
        unsub = fn;
      });

    return () => {
      cancelled = true;
      unsub?.();
    };
  }, []);

  const refresh = useCallback(async () => {
    try {
      setDownloads(await api.listDownloads());
    } catch {
      /* degrade silently */
    }
  }, []);

  const active = downloads.filter((d) => d.status === "active" || d.status === "pending");
  return { downloads, active, refresh };
}
