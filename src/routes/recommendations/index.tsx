import { Link, createFileRoute } from "@tanstack/react-router";
import { useEffect, useMemo, useState } from "react";
import { api } from "@/lib/api/client";
import { useSettingsStore } from "@/lib/stores/settings";
import type { Scene } from "@/lib/types";
import { toWatchMap, watchFor, type WatchMap } from "@/lib/watch";
import { sceneThumbUrl, isVideoScene } from "@/lib/mediaUrl";
import { recommendScenes, resolveRecPrefs } from "@/lib/recommend";
import { SceneCard } from "@/components/SceneCard";
import { ScenePlayerDialog } from "@/components/ScenePlayerDialog";
import { SkeletonGrid } from "@/components/SkeletonGrid";
import { ErrorState } from "@/components/ErrorState";
import { EmptyState } from "@/components/EmptyState";
import { Button } from "@/components/ui/button";
import { Compass, Settings2, Sparkles } from "lucide-react";

export const Route = createFileRoute("/recommendations/")({
  component: RecommendationsPage,
});

function RecommendationsPage() {
  const { settings } = useSettingsStore();
  const [scenes, setScenes] = useState<Scene[]>([]);
  const [watchMap, setWatchMap] = useState<WatchMap>(new Map());
  const [loadError, setLoadError] = useState("");
  const [loading, setLoading] = useState(true);
  const [playerScene, setPlayerScene] = useState<Scene | null>(null);
  const [playerIndex, setPlayerIndex] = useState(0);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setLoading(true);
    setLoadError("");
    const scenesP = api
      .listScenes()
      .then(setScenes)
      .catch((e) => setLoadError(e instanceof Error ? e.message : "Failed to load scenes"));
    const watchP = api
      .listWatchProgress()
      .then((all) => setWatchMap(toWatchMap(all)))
      .catch(() => {});
    Promise.allSettled([scenesP, watchP]).finally(() => setLoading(false));
  }, []);

  const prefs = useMemo(() => resolveRecPrefs(settings), [settings]);
  const recommendations = useMemo(
    () => recommendScenes(scenes, [...watchMap.values()], prefs),
    [scenes, watchMap, prefs],
  );
  const videoScenes = useMemo(() => scenes.filter(isVideoScene), [scenes]);

  function handlePlay(scene: Scene) {
    const idx = videoScenes.findIndex((s) => s.id === scene.id);
    if (idx === -1) return;
    setPlayerIndex(idx);
    setPlayerScene(scene);
  }

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 text-2xl font-bold">
            <Sparkles className="h-5 w-5 text-[var(--color-primary)]" />
            Recommended for you
          </h2>
          <p className="text-sm text-[var(--color-muted-foreground)]">
            Unwatched library scenes scored from your watch history and preferences
          </p>
        </div>
        <Button asChild size="sm" variant="outline">
          <Link to="/settings">
            <Settings2 className="h-4 w-4 mr-2" />
            Tune
          </Link>
        </Button>
      </div>

      {loadError && <ErrorState message={loadError} onRetry={() => window.location.reload()} />}

      {!loadError && !loading && !prefs.enabled && (
        <EmptyState
          icon={<Sparkles className="h-8 w-8" />}
          title="Recommendations are off"
          description="Enable them in Settings to see picks on your Dashboard."
          action={
            <Button asChild size="sm">
              <Link to="/settings">Open Settings</Link>
            </Button>
          }
        />
      )}

      {!loadError && !loading && prefs.enabled && recommendations.length === 0 && (
        <EmptyState
          icon={<Sparkles className="h-8 w-8" />}
          title="Not enough signal yet"
          description="Watch some scenes — or set preferred tags and performers in Settings — and picks will show up here."
          action={
            scenes.length === 0 ? (
              <Button asChild size="sm">
                <Link to="/browse">
                  <Compass className="h-4 w-4 mr-2" />
                  Browse Sites
                </Link>
              </Button>
            ) : (
              <Button asChild size="sm">
                <Link to="/settings">Tune preferences</Link>
              </Button>
            )
          }
        />
      )}

      {!loadError && (loading || recommendations.length > 0) && (
        <>
          {loading && scenes.length === 0 ? (
            <SkeletonGrid count={12} cols={6} />
          ) : (
            <div className="grid grid-cols-3 sm:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
              {recommendations.map(({ scene, reasons }) => (
                <div key={scene.id}>
                  <SceneCard
                    item={scene}
                    thumbSrc={sceneThumbUrl(scene)}
                    watch={watchFor(watchMap, scene.id)}
                    onClick={(item) => handlePlay(item as Scene)}
                  />
                  {reasons.length > 0 && (
                    <p
                      className="mt-1 truncate text-[11px] text-[var(--color-muted-foreground)]"
                      title={reasons.join(" · ")}
                    >
                      {reasons.join(" · ")}
                    </p>
                  )}
                </div>
              ))}
            </div>
          )}
        </>
      )}

      <ScenePlayerDialog
        scene={playerScene}
        scenes={videoScenes}
        currentIndex={playerIndex}
        open={playerScene !== null}
        onClose={() => {
          setPlayerScene(null);
          void api
            .listWatchProgress()
            .then((all) => setWatchMap(toWatchMap(all)))
            .catch(() => {});
        }}
        onNavigate={(scene, idx) => {
          setPlayerScene(scene);
          setPlayerIndex(idx);
        }}
      />
    </div>
  );
}
