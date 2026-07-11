import { Link, createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import { ConnectionStatusChip } from "@/components/ConnectionStatusChip";
import { getCapabilities } from "@/lib/runtime";
import { useSettingsStore } from "@/lib/stores/settings";
import { isMobileDevice } from "@/lib/tauri";
import type { DownloadJob, Scene } from "@/lib/types";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { DownloadProgressRow } from "@/components/DownloadProgress";
import { Button } from "@/components/ui/button";

export const Route = createFileRoute("/")({
  component: HomePage,
});

function HomePage() {
  const { settings } = useSettingsStore();
  const [scenes, setScenes] = useState<Scene[]>([]);
  const [downloads, setDownloads] = useState<DownloadJob[]>([]);
  const [loadError, setLoadError] = useState("");
  const caps = getCapabilities();
  const isMobile = isMobileDevice();
  const needsSetup = (isMobile || caps.showBrowserBanner) && !settings.remote_host;

  useEffect(() => {
    if (needsSetup) return;
    void queueMicrotask(() => setLoadError(""));
    void api
      .listScenes()
      .then(setScenes)
      .catch((e) => setLoadError(e instanceof Error ? e.message : "Failed to load scenes"));
    void api
      .listDownloads()
      .then(setDownloads)
      .catch((e) => setLoadError(e instanceof Error ? e.message : "Failed to load downloads"));
    void api.subscribeDownloadProgress((job) => {
      setDownloads((prev) => {
        const idx = prev.findIndex((j) => j.id === job.id);
        if (idx === -1) return [job, ...prev];
        const next = [...prev];
        next[idx] = job;
        return next;
      });
    });
  }, [needsSetup, settings.remote_host, settings.remote_token]);

  const active = downloads.filter((d) => d.status === "active" || d.status === "pending");

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">Dashboard</h2>
        <p className="text-sm text-[var(--color-muted-foreground)]">
          Recent library items and active downloads
        </p>
      </div>

      {(isMobile || caps.showBrowserBanner) && <ConnectionStatusChip />}

      {caps.showBrowserBanner && !needsSetup && (
        <Card className="border-[var(--color-border)]">
          <CardContent className="p-4 text-sm text-[var(--color-muted-foreground)]">
            Browser mode — API calls go to your configured Remote LAN host on port 8787.
          </CardContent>
        </Card>
      )}

      {needsSetup && (
        <Card className="border-[var(--color-primary)]">
          <CardContent className="space-y-3 p-4 text-sm">
            <p>
              Connect to your desktop ArcHive: open <strong>Settings</strong> → Engine → Remote LAN.
            </p>
            <p className="text-xs text-[var(--color-muted-foreground)]">
              Host: <code>http://192.168.178.69:8787</code> — enable LAN on the PC app first.
            </p>
            <Button asChild size="sm">
              <Link to="/settings">Open Settings</Link>
            </Button>
          </CardContent>
        </Card>
      )}

      {loadError && <p className="text-sm text-[var(--color-destructive)]">{loadError}</p>}

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
