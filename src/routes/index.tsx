import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { DownloadJob, Scene } from "@/lib/types";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { DownloadProgressRow } from "@/components/DownloadProgress";

export const Route = createFileRoute("/")({
  component: HomePage,
});

function HomePage() {
  const [scenes, setScenes] = useState<Scene[]>([]);
  const [downloads, setDownloads] = useState<DownloadJob[]>([]);

  useEffect(() => {
    void api.listScenes().then(setScenes).catch(console.error);
    void api.listDownloads().then(setDownloads).catch(console.error);
    void api.subscribeDownloadProgress((job) => {
      setDownloads((prev) => {
        const idx = prev.findIndex((j) => j.id === job.id);
        if (idx === -1) return [job, ...prev];
        const next = [...prev];
        next[idx] = job;
        return next;
      });
    });
  }, []);

  const active = downloads.filter((d) => d.status === "active" || d.status === "pending");

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">Dashboard</h2>
        <p className="text-sm text-[var(--color-muted-foreground)]">
          Recent library items and active downloads
        </p>
      </div>

      {active.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Active Downloads</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            {active.map((job) => (
              <DownloadProgressRow key={job.id} job={job} />
            ))}
          </CardContent>
        </Card>
      )}

      <div>
        <h3 className="mb-3 text-lg font-semibold">Recent Scenes</h3>
        <div className="grid grid-cols-2 gap-3 md:grid-cols-4 lg:grid-cols-5">
          {scenes.slice(0, 10).map((scene) => (
            <Card key={scene.id} className="overflow-hidden">
              <div className="aspect-video bg-[var(--color-muted)]">
                {scene.thumb && (
                  <img src={scene.thumb} alt={scene.title} className="h-full w-full object-cover" />
                )}
              </div>
              <CardContent className="p-2">
                <p className="line-clamp-2 text-xs font-medium">{scene.title}</p>
              </CardContent>
            </Card>
          ))}
          {scenes.length === 0 && (
            <p className="col-span-full text-sm text-[var(--color-muted-foreground)]">
              No scenes yet. Browse sites or paste a URL to download.
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
