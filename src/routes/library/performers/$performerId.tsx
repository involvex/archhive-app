import { Link, createFileRoute, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { Performer, Scene } from "@/lib/types";
import { CUP_OPTIONS, HAIR_OPTIONS } from "@/lib/body-match/queries";
import { toWatchMap, watchFor, type WatchMap } from "@/lib/watch";
import { SceneCard } from "@/components/SceneCard";
import { ScenePlayerDialog } from "@/components/ScenePlayerDialog";
import { SkeletonGrid } from "@/components/SkeletonGrid";
import { ErrorState } from "@/components/ErrorState";
import { EmptyState } from "@/components/EmptyState";
import { Button } from "@/components/ui/button";
import { sceneThumbUrl, isVideoScene } from "@/lib/mediaUrl";
import { ArrowLeft, Filter, Users } from "lucide-react";

export const Route = createFileRoute("/library/performers/$performerId")({
  component: PerformerDetailPage,
});

function PerformerDetailPage() {
  const { performerId } = Route.useParams();
  const navigate = useNavigate();
  const [performer, setPerformer] = useState<Performer | null>(null);
  const [scenes, setScenes] = useState<Scene[]>([]);
  const [watchMap, setWatchMap] = useState<WatchMap>(new Map());
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [playerScene, setPlayerScene] = useState<Scene | null>(null);
  const [playerIndex, setPlayerIndex] = useState(0);
  const [cupEdit, setCupEdit] = useState("");
  const [hairEdit, setHairEdit] = useState("");
  const [attrSaving, setAttrSaving] = useState(false);
  const [attrError, setAttrError] = useState("");

  useEffect(() => {
    let cancelled = false;
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const all = await api.listPerformers();
        const found = all.find((p) => p.id === performerId) ?? null;
        if (cancelled) return;
        setPerformer(found);
        setCupEdit(found?.cup_size ?? "");
        setHairEdit(found?.hair_color ?? "");
        setAttrError("");
        if (!found) {
          setScenes([]);
          return;
        }
        const [filtered, progress] = await Promise.all([
          api.listScenesWithFilter({ performer_names: [found.name] }),
          api.listWatchProgress().catch(() => []),
        ]);
        if (cancelled) return;
        setScenes(filtered);
        setWatchMap(toWatchMap(progress));
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : "Failed to load performer");
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    void load();
    return () => {
      cancelled = true;
    };
  }, [performerId]);

  const videoScenes = scenes.filter(isVideoScene);

  function handlePlay(scene: Scene) {
    const idx = videoScenes.findIndex((s) => s.id === scene.id);
    if (idx === -1) {
      setPlayerScene(scene);
      setPlayerIndex(0);
      return;
    }
    setPlayerIndex(idx);
    setPlayerScene(scene);
  }

  if (loading) {
    return (
      <div className="space-y-4">
        <SkeletonGrid count={6} cols={3} />
      </div>
    );
  }

  if (error) {
    return <ErrorState message={error} onRetry={() => navigate({ to: "/library/performers" })} />;
  }

  if (!performer) {
    return (
      <EmptyState
        icon={<Users className="h-12 w-12" />}
        title="Performer not found"
        description="They may have been renamed or removed from the library."
        action={
          <Button asChild size="sm">
            <Link to="/library/performers">Back to performers</Link>
          </Button>
        }
      />
    );
  }

  return (
    <div className="space-y-4">
      <Button variant="ghost" size="sm" onClick={() => navigate({ to: "/library/performers" })}>
        <ArrowLeft className="mr-1.5 h-4 w-4" />
        Performers
      </Button>
      <div className="flex items-center gap-4">
        <div className="flex h-16 w-16 shrink-0 items-center justify-center overflow-hidden rounded-full bg-[var(--color-muted)]">
          {performer.image ? (
            <img
              src={performer.image}
              alt={performer.name}
              className="h-full w-full object-cover"
            />
          ) : (
            <Users className="h-7 w-7" />
          )}
        </div>
        <div className="min-w-0 flex-1">
          <h2 className="truncate text-2xl font-bold">{performer.name}</h2>
          <p className="text-sm text-[var(--color-muted-foreground)]">
            {performer.scene_count} scene{performer.scene_count === 1 ? "" : "s"}
            {performer.aliases.length > 0 && ` · aka ${performer.aliases.join(", ")}`}
          </p>
        </div>
        <Button
          size="sm"
          variant="outline"
          onClick={() =>
            navigate({ to: "/library/scenes", search: { performers: [performer.name] } })
          }
        >
          <Filter className="mr-1.5 h-3.5 w-3.5" />
          Filter scenes
        </Button>
      </div>
      <div className="flex flex-col gap-2 rounded-lg border border-[var(--color-border)] bg-[var(--color-card)] p-3 sm:flex-row sm:items-end">
        <label className="flex flex-1 flex-col gap-1 text-xs font-medium text-[var(--color-muted-foreground)]">
          Cup size
          <select
            className="h-9 rounded-md border border-[var(--color-border)] bg-[var(--color-background)] px-2 text-sm text-[var(--color-foreground)]"
            value={cupEdit}
            onChange={(e) => setCupEdit(e.target.value)}
          >
            <option value="">Unset</option>
            {CUP_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-1 flex-col gap-1 text-xs font-medium text-[var(--color-muted-foreground)]">
          Hair color
          <select
            className="h-9 rounded-md border border-[var(--color-border)] bg-[var(--color-background)] px-2 text-sm text-[var(--color-foreground)]"
            value={hairEdit}
            onChange={(e) => setHairEdit(e.target.value)}
          >
            <option value="">Unset</option>
            {HAIR_OPTIONS.filter((o) => o.value !== "any").map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
        </label>
        <Button
          size="sm"
          disabled={attrSaving}
          onClick={() => {
            setAttrSaving(true);
            setAttrError("");
            void api
              .updatePerformerAttributes(performer.id, {
                cup_size: cupEdit || null,
                hair_color: hairEdit || null,
              })
              .then(() =>
                setPerformer((prev) =>
                  prev
                    ? {
                        ...prev,
                        cup_size: cupEdit || undefined,
                        hair_color: hairEdit || undefined,
                      }
                    : prev,
                ),
              )
              .catch((e) => setAttrError(e instanceof Error ? e.message : "Save failed"))
              .finally(() => setAttrSaving(false));
          }}
        >
          {attrSaving ? "Saving…" : "Save attributes"}
        </Button>
      </div>
      {attrError && <p className="text-xs text-red-400">{attrError}</p>}
      {scenes.length === 0 ? (
        <EmptyState
          icon={<Users className="h-12 w-12" />}
          title="No scenes found"
          description={`${performer.name} has no scenes in the library right now.`}
        />
      ) : (
        <div className="grid grid-cols-3 sm:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
          {scenes.map((scene) => (
            <SceneCard
              key={scene.id}
              item={scene}
              thumbSrc={sceneThumbUrl(scene)}
              watch={watchFor(watchMap, scene.id)}
              onClick={(item) => handlePlay(item as Scene)}
            />
          ))}
        </div>
      )}
      <ScenePlayerDialog
        scene={playerScene}
        scenes={videoScenes}
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
