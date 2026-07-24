import { createFileRoute } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { RotateCcw } from "lucide-react";
import { api } from "@/lib/api/client";
import type { DownloadJob } from "@/lib/types";
import { isDesktopTauri } from "@/lib/tauri";
import { DownloadProgressRow } from "@/components/DownloadProgress";
import { BulkImportPanel } from "@/components/BulkImportPanel";
import { Button } from "@/components/ui/button";

export const Route = createFileRoute("/downloads/")({
  component: DownloadsPage,
});

function DownloadsPage() {
  const [jobs, setJobs] = useState<DownloadJob[]>([]);

  const refreshJobs = useCallback(() => {
    void api.listDownloads().then(setJobs).catch(console.error);
  }, []);

  const failedIds = useMemo(
    () => jobs.filter((j) => j.status === "failed" || j.status === "cancelled").map((j) => j.id),
    [jobs],
  );

  const retryFailed = useCallback(async () => {
    for (const id of failedIds) {
      try {
        await api.retryDownload(id);
      } catch (e) {
        console.error("retry failed", id, e);
      }
    }
    refreshJobs();
  }, [failedIds, refreshJobs]);

  useEffect(() => {
    refreshJobs();
    const unsubs: Array<() => void> = [];
    void api
      .subscribeDownloadProgress((job) => {
        setJobs((prev) => {
          const idx = prev.findIndex((j) => j.id === job.id);
          if (idx === -1) return [job, ...prev];
          const next = [...prev];
          next[idx] = job;
          return next;
        });
      })
      .then((fn) => unsubs.push(fn));
    if (isDesktopTauri()) {
      void listen<string>("download:deleted", (e) => {
        const id = e.payload;
        setJobs((prev) => prev.filter((j) => j.id !== id));
      }).then((fn) => unsubs.push(fn));
    }
    return () => {
      for (const fn of unsubs) fn();
    };
  }, [refreshJobs]);

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-2xl font-bold">Downloads</h2>
        {failedIds.length > 0 && (
          <Button variant="outline" size="sm" onClick={() => void retryFailed()}>
            <RotateCcw className="mr-1.5 h-4 w-4" />
            Retry failed ({failedIds.length})
          </Button>
        )}
      </div>
      <BulkImportPanel onQueued={refreshJobs} />
      <div className="space-y-2">
        {jobs.map((job) => (
          <DownloadProgressRow
            key={job.id}
            job={job}
            onPause={(id) => void api.pauseDownload(id).then(refreshJobs)}
            onResume={(id) => void api.resumeDownload(id).then(refreshJobs)}
            onRetry={(id) => void api.retryDownload(id).then(refreshJobs)}
            onCancel={(id) => void api.cancelDownload(id).then(refreshJobs)}
            onDelete={(id) => void api.deleteDownload(id).then(refreshJobs)}
          />
        ))}
        {jobs.length === 0 && (
          <p className="text-sm text-[var(--color-muted-foreground)]">No downloads yet.</p>
        )}
      </div>
    </div>
  );
}
