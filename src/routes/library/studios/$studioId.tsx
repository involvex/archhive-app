import { Link, createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { Scene, Studio } from "@/lib/types";
import { toWatchMap, watchFor, type WatchMap } from "@/lib/watch";
import { SceneCard } from "@/components/SceneCard";
import { ScenePlayerDialog } from "@/components/ScenePlayerDialog";
import { SceneDetailsDialog } from "@/components/SceneDetailsDialog";
import { SkeletonGrid } from "@/components/SkeletonGrid";
import { ErrorState } from "@/components/ErrorState";
import { EmptyState } from "@/components/EmptyState";
import { sceneThumbUrl, isVideoScene } from "@/lib/mediaUrl";
import { ArrowLeft, Clapperboard } from "lucide-react";

export const Route = createFileRoute("/library/studios/$studioId")({
  component: StudioDetailPage,
});

function StudioDetailPage() {
  const { studioId } = Route.useParams();
  const [studio, setStudio] = useState<Studio | null>(null);
  const [scenes, setScenes] = useState<Scene[]>([]);
  const [watchMap, setWatchMap] = useState<WatchMap>(new Map());
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [playerScene, setPlayerScene] = useState<Scene | null>(null);
  const [playerIndex, setPlayerIndex] = useState(0);
  const [detailsScene, setDetailsScene] = useState<Scene | null>(null);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const [allStudios, studioScenes, progress] = await Promise.all([
          api.listStudios(),
          api.listScenesForStudio(studioId),
          api.listWatchProgress().catch(() => []),
        ]);
        if (cancelled) return;
        setStudio(allStudios.find((s) => s.id === studioId) ?? null);
        setScenes(studioScenes);
        setWatchMap(toWatchMap(progress));
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : "Failed to load studio");
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    void load();
    return () => {
      cancelled = true;
    };
  }, [studioId]);

  if (loading) {
    return (
      <div className="space-y-4">
        <SkeletonGrid count={8} cols={3} />
      </div>
    );
  }

  if (error) {
    return <ErrorState message={error} />;
  }

  const title = studio?.name ?? "Studio";
  const videoScenes = scenes.filter(isVideoScene);

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-3">
        <Link
          to="/library/studios"
          className="rounded-md p-1 text-[var(--color-muted-foreground)] hover:bg-[var(--color-muted)]"
          aria-label="Back to studios"
        >
          <ArrowLeft className="h-5 w-5" />
        </Link>
        <div>
          <h2 className="text-2xl font-bold">{title}</h2>
          <p className="text-sm text-[var(--color-muted-foreground)]">
            {scenes.length} scene{scenes.length !== 1 ? "s" : ""}
          </p>
        </div>
      </div>

      {scenes.length === 0 ? (
        <EmptyState
          icon={<Clapperboard className="h-12 w-12" />}
          title="No scenes"
          description="This studio has no matching library scenes."
        />
      ) : (
        <div className="grid grid-cols-3 sm:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
          {scenes.map((scene) => {
            const canPlay = isVideoScene(scene);
            return (
              <button
                key={scene.id}
                type="button"
                className="w-full text-left [appearance:button] bg-transparent border-0 p-0"
                onClick={() => {
                  if (canPlay) {
                    const idx = videoScenes.findIndex((v) => v.id === scene.id);
                    setPlayerIndex(idx >= 0 ? idx : 0);
                    setPlayerScene(scene);
                  } else {
                    setDetailsScene(scene);
                  }
                }}
              >
                <SceneCard
                  item={scene}
                  thumbSrc={sceneThumbUrl(scene)}
                  selected={false}
                  selectionMode={false}
                  watch={watchFor(watchMap, scene.id)}
                  onClick={() => {}}
                />
              </button>
            );
          })}
        </div>
      )}

      <ScenePlayerDialog
        scene={playerScene}
        scenes={videoScenes}
        currentIndex={playerIndex}
        open={playerScene !== null}
        onClose={() => setPlayerScene(null)}
        onNavigate={(s, i) => {
          setPlayerScene(s);
          setPlayerIndex(i);
        }}
      />

      <SceneDetailsDialog
        scene={detailsScene}
        open={detailsScene !== null}
        onClose={() => setDetailsScene(null)}
      />
    </div>
  );
}
