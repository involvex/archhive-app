import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useMemo, useState } from "react";
import { api } from "@/lib/api/client";
import type { Scene, WatchProgress } from "@/lib/types";
import { toWatchMap, watchFor, type WatchMap } from "@/lib/watch";
import { SceneCard } from "@/components/SceneCard";
import { ScenePlayerDialog } from "@/components/ScenePlayerDialog";
import { SkeletonGrid } from "@/components/SkeletonGrid";
import { ErrorState } from "@/components/ErrorState";
import { EmptyState } from "@/components/EmptyState";
import { sceneThumbUrl, isVideoScene } from "@/lib/mediaUrl";
import { History } from "lucide-react";

export const Route = createFileRoute("/library/history/")({
  component: HistoryPage,
});

function formatRelative(iso: string): string {
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return "";
  const mins = Math.max(0, Math.floor((Date.now() - then) / 60000));
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d ago`;
  const months = Math.floor(days / 30);
  if (months < 12) return `${months}mo ago`;
  return `${Math.floor(months / 12)}y ago`;
}

function HistoryPage() {
  const [progress, setProgress] = useState<WatchProgress[]>([]);
  const [scenesById, setScenesById] = useState<Map<string, Scene>>(new Map());
  const [watchMap, setWatchMap] = useState<WatchMap>(new Map());
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [playerScene, setPlayerScene] = useState<Scene | null>(null);
  const [playerIndex, setPlayerIndex] = useState(0);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const [all, scenes] = await Promise.all([
          api.listWatchProgress(),
          api.listScenes().catch(() => [] as Scene[]),
        ]);
        if (cancelled) return;
        const sorted = [...all].sort(
          (a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime(),
        );
        setProgress(sorted);
        setWatchMap(toWatchMap(sorted));
        setScenesById(new Map(scenes.map((s) => [s.id, s])));
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : "Failed to load history");
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    void load();
    return () => {
      cancelled = true;
    };
  }, []);

  const entries = useMemo(
    () =>
      progress
        .map((w) => ({ row: w, scene: scenesById.get(w.scene_id) }))
        .filter((e): e is { row: WatchProgress; scene: Scene } => e.scene != null),
    [progress, scenesById],
  );
  const inProgress = useMemo(() => entries.filter((e) => !e.row.watched), [entries]);
  const watched = useMemo(() => entries.filter((e) => e.row.watched), [entries]);

  const playable = useMemo(() => entries.map((e) => e.scene).filter(isVideoScene), [entries]);

  function handlePlay(scene: Scene) {
    const idx = playable.findIndex((s) => s.id === scene.id);
    if (idx === -1) {
      setPlayerScene(scene);
      setPlayerIndex(0);
      return;
    }
    setPlayerIndex(idx);
    setPlayerScene(scene);
  }

  function renderGrid(list: { row: WatchProgress; scene: Scene }[]) {
    return (
      <div className="grid grid-cols-3 sm:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
        {list.map(({ row, scene }) => (
          <div key={scene.id} className="relative">
            <SceneCard
              item={scene}
              thumbSrc={sceneThumbUrl(scene)}
              watch={watchFor(watchMap, scene.id)}
              onClick={(item) => handlePlay(item as Scene)}
            />
            <p className="mt-1 truncate text-[11px] text-[var(--color-muted-foreground)]">
              {formatRelative(row.updated_at)}
            </p>
          </div>
        ))}
      </div>
    );
  }

  if (loading) {
    return (
      <div className="space-y-4">
        <h2 className="text-2xl font-bold">History</h2>
        <SkeletonGrid count={12} cols={6} />
      </div>
    );
  }

  if (error) {
    return (
      <div className="space-y-4">
        <h2 className="text-2xl font-bold">History</h2>
        <ErrorState message={error} onRetry={() => window.location.reload()} />
      </div>
    );
  }

  if (entries.length === 0) {
    return (
      <div className="space-y-4">
        <h2 className="text-2xl font-bold">History</h2>
        <EmptyState
          icon={<History className="h-12 w-12" />}
          title="No watch history yet"
          description="Scenes you play will show up here so you can resume where you left off."
        />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold">History</h2>
      {inProgress.length > 0 && (
        <section className="space-y-3">
          <h3 className="text-lg font-semibold">
            In progress{" "}
            <span className="text-sm font-normal text-[var(--color-muted-foreground)]">
              {inProgress.length}
            </span>
          </h3>
          {renderGrid(inProgress)}
        </section>
      )}
      {watched.length > 0 && (
        <section className="space-y-3">
          <h3 className="text-lg font-semibold">
            Watched{" "}
            <span className="text-sm font-normal text-[var(--color-muted-foreground)]">
              {watched.length}
            </span>
          </h3>
          {renderGrid(watched)}
        </section>
      )}
      <ScenePlayerDialog
        scene={playerScene}
        scenes={playable}
        currentIndex={playerIndex}
        open={playerScene !== null}
        onClose={() => setPlayerScene(null)}
        onNavigate={(scene, idx) => {
          setPlayerScene(scene);
          setPlayerIndex(idx);
        }}
      />
    </div>
  );
}
