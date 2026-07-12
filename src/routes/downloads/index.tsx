import { createFileRoute } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "@/lib/api/client";
import type { DownloadJob } from "@/lib/types";
import { isDesktopTauri } from "@/lib/tauri";
import { DownloadProgressRow } from "@/components/DownloadProgress";
import { BulkImportPanel } from "@/components/BulkImportPanel";

export const Route = createFileRoute("/downloads/")({
  component: DownloadsPage,
});

function DownloadsPage() {
  const [jobs, setJobs] = useState<DownloadJob[]>([]);

  const refreshJobs = useCallback(() => {
    void api.listDownloads().then(setJobs).catch(console.error);
  }, []);

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
      <h2 className="text-2xl font-bold">Downloads</h2>
      <BulkImportPanel onQueued={refreshJobs} />
      <div className="space-y-2">
        {jobs.map((job) => (
          <DownloadProgressRow
            key={job.id}
            job={job}
            onPause={(id) => void api.pauseDownload(id).then(refreshJobs)}
            onResume={(id) => void api.resumeDownload(id).then(refreshJobs)}
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
