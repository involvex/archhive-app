import { Link, createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import { ConnectionStatusChip } from "@/components/ConnectionStatusChip";
import { getCapabilities } from "@/lib/runtime";
import { useSettingsStore } from "@/lib/stores/settings";
import { isMobileDevice } from "@/lib/tauri";
import type { DownloadJob, Scene, WatchProgress } from "@/lib/types";
import type { CardWatchState } from "@/components/SceneCard";
import { sceneThumbUrl, isVideoScene } from "@/lib/mediaUrl";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { DownloadProgressRow } from "@/components/DownloadProgress";
import { Button } from "@/components/ui/button";
import { ScenePlayerDialog } from "@/components/ScenePlayerDialog";
import { SkeletonGrid } from "@/components/SkeletonGrid";
import { ErrorState } from "@/components/ErrorState";
import { EmptyState } from "@/components/EmptyState";
import { SceneCard } from "@/components/SceneCard";
import { useRecentlyViewedStore } from "@/lib/stores/recentlyViewed";
import { Compass, Link2, Radio, Film, History, Newspaper, X } from "lucide-react";
import { PORNHUB_FEED_SLUG } from "@/lib/sites/catalog";

export const Route = createFileRoute("/")({
  component: HomePage,
});

function HomePage() {
  const { settings } = useSettingsStore();
  const [scenes, setScenes] = useState<Scene[]>([]);
  const [downloads, setDownloads] = useState<DownloadJob[]>([]);
  const [loadError, setLoadError] = useState("");
  const [loading, setLoading] = useState(true);
  const [playerScene, setPlayerScene] = useState<Scene | null>(null);
  const [playerIndex, setPlayerIndex] = useState(0);
  // #26 watch-history map for card overlays.
  const [watchMap, setWatchMap] = useState<Map<string, WatchProgress>>(new Map());
  const caps = getCapabilities();
  const isMobile = isMobileDevice();
  const needsSetup = (isMobile || caps.showBrowserBanner) && !settings.remote_host;

  useEffect(() => {
    if (needsSetup) return;
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setLoading(true);
    setLoadError("");

    const scenesP = api
      .listScenes()
      .then(setScenes)
      .catch((e) => setLoadError(e instanceof Error ? e.message : "Failed to load scenes"));
    void api
      .listWatchProgress()
      .then((all) => setWatchMap(new Map(all.map((w) => [w.scene_id, w]))))
      .catch(() => {});
    const downloadsP = api
      .listDownloads()
      .then(setDownloads)
      .catch((e) => setLoadError(e instanceof Error ? e.message : "Failed to load downloads"));

    Promise.allSettled([scenesP, downloadsP]).finally(() => setLoading(false));
  }, [needsSetup]);

  useEffect(() => {
    let unsubDownload: (() => void) | undefined;
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
        unsubDownload = fn;
      });
    return () => {
      unsubDownload?.();
    };
  }, [needsSetup]);

  const active = downloads.filter((d) => d.status === "active" || d.status === "pending");
  const recent = useRecentlyViewedStore((s) => s.recent);
  const clearRecent = useRecentlyViewedStore((s) => s.clear);

  function watchFor(id: string): CardWatchState | null {
    const w = watchMap.get(id);
    if (!w) return null;
    return { position: w.position_secs, duration: w.duration_secs, watched: w.watched };
  }

  function refreshWatch() {
    void api
      .listWatchProgress()
      .then((all) => setWatchMap(new Map(all.map((w) => [w.scene_id, w]))))
      .catch(() => {});
  }

  const videoScenes = scenes.filter(isVideoScene);
  // Rail shows stored snapshots; fall back to the live scene object when available.
  const recentVisible = recent.slice(0, 12);
  function handlePlayRecent(snapshot: Scene) {
    const live = videoScenes.find((s) => s.id === snapshot.id) ?? snapshot;
    if (!isVideoScene(live)) {
      setPlayerScene(live);
      setPlayerIndex(0);
      return;
    }
    handlePlay(live);
  }

  function handlePlay(scene: Scene) {
    const idx = videoScenes.findIndex((s) => s.id === scene.id);
    if (idx === -1) return;
    setPlayerIndex(idx);
    setPlayerScene(scene);
  }

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

      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
        <Card>
          <CardContent className="p-4">
            <p className="text-xs text-[var(--color-muted-foreground)]">Total Scenes</p>
            <p className="text-2xl font-bold tabular-nums">{scenes.length}</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-4">
            <p className="text-xs text-[var(--color-muted-foreground)]">Downloads</p>
            <p className="text-2xl font-bold tabular-nums">{downloads.length}</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-4">
            <p className="text-xs text-[var(--color-muted-foreground)]">Active</p>
            <p className="text-2xl font-bold tabular-nums">{active.length}</p>
          </CardContent>
        </Card>
        <Link to="/library/scenes" className="block">
          <Card className="hover:border-[var(--color-primary)] transition-colors cursor-pointer">
            <CardContent className="p-4">
              <p className="text-xs text-[var(--color-muted-foreground)]">Library</p>
              <p className="text-2xl font-bold">{scenes.length}</p>
            </CardContent>
          </Card>
        </Link>
      </div>

      <div className="flex flex-wrap gap-2">
        <Button asChild size="sm" variant="secondary">
          <Link to="/browse">
            <Compass className="h-4 w-4 mr-2" />
            Browse Sites
          </Link>
        </Button>
        <Button asChild size="sm" variant="secondary">
          <Link to="/browse/by-url">
            <Link2 className="h-4 w-4 mr-2" />
            Paste URL
          </Link>
        </Button>
        <Button asChild size="sm" variant="secondary">
          <Link to="/live">
            <Radio className="h-4 w-4 mr-2" />
            Live Streams
          </Link>
        </Button>
      </div>

      <Card>
        <CardContent className="p-4 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="rounded-md bg-[var(--color-primary)]/10 p-2">
              <Newspaper className="h-5 w-5 text-[var(--color-primary)]" />
            </div>
            <div>
              <p className="text-sm font-medium">News / Feed</p>
              <p className="text-xs text-[var(--color-muted-foreground)]">
                Browse promoted and featured content from PornHub
              </p>
            </div>
          </div>
          <Button asChild size="sm" variant="outline">
            <Link
              to="/browse/$site/$kind/$slug"
              params={{ site: "pornhub", kind: "search", slug: PORNHUB_FEED_SLUG }}
            >
              Browse Feed
            </Link>
          </Button>
        </CardContent>
      </Card>

      {active.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              Active Downloads
              <span className="rounded-full bg-[var(--color-primary)]/20 px-2 py-0.5 text-xs text-[var(--color-primary)]">
                {active.length} Active
              </span>
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            {active.map((job) => (
              <DownloadProgressRow key={job.id} job={job} />
            ))}
          </CardContent>
        </Card>
      )}

      {recentVisible.length > 0 && (
        <div>
          <div className="mb-3 flex items-center justify-between">
            <h3 className="flex items-center gap-2 text-lg font-semibold">
              <History className="h-4 w-4 text-[var(--color-muted-foreground)]" />
              Continue watching
            </h3>
            <button
              type="button"
              onClick={clearRecent}
              className="flex items-center gap-1 text-xs text-[var(--color-muted-foreground)] hover:text-[var(--color-foreground)]"
              title="Clear recently played"
            >
              <X className="h-3 w-3" />
              Clear
            </button>
          </div>
          <div className="grid grid-cols-3 sm:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
            {recentVisible.map((snapshot) => (
              <SceneCard
                key={snapshot.id}
                item={snapshot}
                thumbSrc={sceneThumbUrl(snapshot)}
                watch={watchFor(snapshot.id)}
                onClick={(item) => handlePlayRecent(item as Scene)}
              />
            ))}
          </div>
        </div>
      )}

      <div>
        <h3 className="mb-3 text-lg font-semibold">Recent Scenes</h3>
        {loading && !scenes.length && !loadError ? (
          <SkeletonGrid count={12} cols={6} />
        ) : loadError ? (
          <ErrorState
            message={loadError}
            onRetry={() => {
              setLoading(true);
              setLoadError("");
              Promise.allSettled([
                api.listScenes().then(setScenes),
                api.listDownloads().then(setDownloads),
              ])
                .catch((e) => setLoadError(e instanceof Error ? e.message : "Failed to load data"))
                .finally(() => setLoading(false));
            }}
          />
        ) : scenes.length === 0 ? (
          <EmptyState
            icon={<Film className="h-8 w-8" />}
            title="No Scenes Yet"
            description="Browse sites or paste a URL to start downloading media."
            action={
              <Button asChild size="sm">
                <Link to="/browse">
                  <Compass className="h-4 w-4 mr-2" />
                  Browse Sites
                </Link>
              </Button>
            }
          />
        ) : (
          <div className="grid grid-cols-3 sm:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
            {scenes.slice(0, 12).map((scene) => (
              <SceneCard
                key={scene.id}
                item={scene}
                thumbSrc={sceneThumbUrl(scene)}
                watch={watchFor(scene.id)}
                onClick={(item) => handlePlay(item as Scene)}
              />
            ))}
          </div>
        )}
      </div>

      <ScenePlayerDialog
        scene={playerScene}
        scenes={videoScenes}
        currentIndex={playerIndex}
        open={playerScene !== null}
        onClose={() => {
          setPlayerScene(null);
          refreshWatch();
        }}
        onNavigate={(scene, idx) => {
          setPlayerScene(scene);
          setPlayerIndex(idx);
        }}
      />
    </div>
  );
}
