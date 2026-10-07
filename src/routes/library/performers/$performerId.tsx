import { Link, createFileRoute, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { Performer, Scene, StashPerformerMatch } from "@/lib/types";
import { CUP_OPTIONS, HAIR_OPTIONS } from "@/lib/body-match/queries";
import { toWatchMap, watchFor, type WatchMap } from "@/lib/watch";
import { SceneCard } from "@/components/SceneCard";
import { ScenePlayerDialog } from "@/components/ScenePlayerDialog";
import { SkeletonGrid } from "@/components/SkeletonGrid";
import { ErrorState } from "@/components/ErrorState";
import { EmptyState } from "@/components/EmptyState";
import { Button } from "@/components/ui/button";
import { sceneThumbUrl, isVideoScene } from "@/lib/mediaUrl";
import { ArrowLeft, Check, Database, Filter, Users, X } from "lucide-react";

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
  // StashDB performer linking.
  const [stashMatches, setStashMatches] = useState<StashPerformerMatch[] | null>(null);
  const [stashSearching, setStashSearching] = useState(false);
  const [stashLinking, setStashLinking] = useState<string | null>(null);
  const [stashError, setStashError] = useState("");

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
      <div className="flex flex-col gap-2 rounded-lg border border-[var(--color-border)] bg-[var(--color-card)] p-3">
        <div className="flex items-center justify-between gap-2">
          <p className="text-xs font-medium text-[var(--color-muted-foreground)]">
            StashDB
            {performer.stash_id ? (
              <span className="ml-2 rounded bg-green-500/15 px-1.5 py-0.5 text-green-400">
                linked
              </span>
            ) : (
              <span className="ml-2 rounded bg-[var(--color-muted)] px-1.5 py-0.5">not linked</span>
            )}
          </p>
          {!performer.stash_id && (
            <Button
              size="sm"
              variant="outline"
              disabled={stashSearching}
              onClick={() => {
                setStashSearching(true);
                setStashError("");
                void api
                  .searchStashdbPerformers(performer.name)
                  .then((matches) => {
                    setStashMatches(matches);
                    if (matches.length === 0) setStashError("No StashDB match for this name.");
                  })
                  .catch((e) => setStashError(e instanceof Error ? e.message : "Search failed"))
                  .finally(() => setStashSearching(false));
              }}
            >
              <Database className="mr-1.5 h-3.5 w-3.5" />
              {stashSearching ? "Searching…" : "Search StashDB"}
            </Button>
          )}
        </div>
        {stashError && <p className="text-xs text-red-400">{stashError}</p>}
        {stashMatches && stashMatches.length > 0 && !performer.stash_id && (
          <ul className="space-y-2">
            {stashMatches.map((m) => (
              <li
                key={m.stash_id}
                className="flex items-center justify-between gap-2 rounded-md border border-[var(--color-border)] p-2 text-sm"
              >
                <div className="flex min-w-0 items-center gap-2">
                  {m.image ? (
                    <img
                      src={m.image}
                      alt=""
                      className="h-8 w-8 shrink-0 rounded-full object-cover"
                    />
                  ) : (
                    <Users className="h-5 w-5 shrink-0 text-[var(--color-muted-foreground)]" />
                  )}
                  <div className="min-w-0">
                    <p className="truncate font-medium">{m.name}</p>
                    {m.aliases.length > 0 && (
                      <p className="truncate text-xs text-[var(--color-muted-foreground)]">
                        aka {m.aliases.join(", ")}
                      </p>
                    )}
                  </div>
                </div>
                <Button
                  size="sm"
                  variant="outline"
                  disabled={stashLinking !== null}
                  onClick={() => {
                    setStashLinking(m.stash_id);
                    setStashError("");
                    void api
                      .linkPerformerStash({
                        performerId: performer.id,
                        stashId: m.stash_id,
                        image: m.image,
                        aliases: m.aliases,
                      })
                      .then((updated) => {
                        setPerformer(updated);
                        setStashMatches(null);
                      })
                      .catch((e) => setStashError(e instanceof Error ? e.message : "Link failed"))
                      .finally(() => setStashLinking(null));
                  }}
                >
                  {stashLinking === m.stash_id ? (
                    "Linking…"
                  ) : (
                    <>
                      <Check className="mr-1 h-3.5 w-3.5" />
                      Apply
                    </>
                  )}
                </Button>
              </li>
            ))}
          </ul>
        )}
        {stashMatches && (
          <button
            type="button"
            onClick={() => setStashMatches(null)}
            className="flex items-center gap-1 self-start text-xs text-[var(--color-muted-foreground)] underline"
          >
            <X className="h-3 w-3" />
            Clear results
          </button>
        )}
      </div>
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
