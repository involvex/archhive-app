import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { DownloadJob } from "@/lib/types";
import { DownloadProgressRow } from "@/components/DownloadProgress";

export const Route = createFileRoute("/downloads/")({
  component: DownloadsPage,
});

function DownloadsPage() {
  const [jobs, setJobs] = useState<DownloadJob[]>([]);

  useEffect(() => {
    void api.listDownloads().then(setJobs).catch(console.error);
    void api.subscribeDownloadProgress((job) => {
      setJobs((prev) => {
        const idx = prev.findIndex((j) => j.id === job.id);
        if (idx === -1) return [job, ...prev];
        const next = [...prev];
        next[idx] = job;
        return next;
      });
    });
  }, []);

  return (
    <div className="space-y-4">
      <h2 className="text-2xl font-bold">Downloads</h2>
      <div className="space-y-2">
        {jobs.map((job) => (
          <DownloadProgressRow
            key={job.id}
            job={job}
            onCancel={(id) => void api.cancelDownload(id)}
          />
        ))}
        {jobs.length === 0 && (
          <p className="text-sm text-[var(--color-muted-foreground)]">No downloads yet.</p>
        )}
      </div>
    </div>
  );
}
