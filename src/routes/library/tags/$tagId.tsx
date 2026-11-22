import { Link, createFileRoute, useNavigate } from "@tanstack/react-router";
import { useEffect, useMemo, useState } from "react";
import { api } from "@/lib/api/client";
import type { Scene, Tag } from "@/lib/types";
import { toWatchMap, watchFor, type WatchMap } from "@/lib/watch";
import { SceneCard } from "@/components/SceneCard";
import { ScenePlayerDialog } from "@/components/ScenePlayerDialog";
import { SkeletonGrid } from "@/components/SkeletonGrid";
import { ErrorState } from "@/components/ErrorState";
import { EmptyState } from "@/components/EmptyState";
import { Button } from "@/components/ui/button";
import { sceneThumbUrl, isVideoScene } from "@/lib/mediaUrl";
import { ArrowLeft, Filter, Tags } from "lucide-react";

export const Route = createFileRoute("/library/tags/$tagId")({
  component: TagDetailPage,
});

function TagDetailPage() {
  const { tagId } = Route.useParams();
  const navigate = useNavigate();
  const [tag, setTag] = useState<Tag | null>(null);
  const [allTags, setAllTags] = useState<Tag[]>([]);
  const [scenes, setScenes] = useState<Scene[]>([]);
  const [watchMap, setWatchMap] = useState<WatchMap>(new Map());
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [playerScene, setPlayerScene] = useState<Scene | null>(null);
  const [playerIndex, setPlayerIndex] = useState(0);
  const [parentEdit, setParentEdit] = useState("");
  const [parentSaving, setParentSaving] = useState(false);
  const [parentError, setParentError] = useState("");

  useEffect(() => {
    let cancelled = false;
    async function load() {
      setLoading(true);
      setError(null);
      try {
        const all = await api.listTags();
        const found = all.find((t) => t.id === tagId) ?? null;
        if (cancelled) return;
        setAllTags(all);
        setTag(found);
        setParentEdit(found?.parent_id ?? "");
        setParentError("");
        if (!found) {
          setScenes([]);
          return;
        }
        const [filtered, progress] = await Promise.all([
          api.listScenesWithFilter({ tag_names: [found.name] }),
          api.listWatchProgress().catch(() => []),
        ]);
        if (cancelled) return;
        setScenes(filtered);
        setWatchMap(toWatchMap(progress));
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : "Failed to load tag");
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    void load();
    return () => {
      cancelled = true;
    };
  }, [tagId]);

  const children = useMemo(
    () => allTags.filter((t) => t.parent_id === tagId).sort((a, b) => a.name.localeCompare(b.name)),
    [allTags, tagId],
  );
  const parentName = useMemo(
    () => allTags.find((t) => t.id === tag?.parent_id)?.name,
    [allTags, tag?.parent_id],
  );

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
    return <ErrorState message={error} onRetry={() => navigate({ to: "/library/tags" })} />;
  }

  if (!tag) {
    return (
      <EmptyState
        icon={<Tags className="h-12 w-12" />}
        title="Tag not found"
        description="It may have been renamed or removed from the library."
        action={
          <Button asChild size="sm">
            <Link to="/library/tags">Back to tags</Link>
          </Button>
        }
      />
    );
  }

  return (
    <div className="space-y-4">
      <Button variant="ghost" size="sm" onClick={() => navigate({ to: "/library/tags" })}>
        <ArrowLeft className="mr-1.5 h-4 w-4" />
        Tags
      </Button>
      <div className="flex items-center gap-4">
        <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-full bg-[var(--color-primary)]/10">
          <Tags className="h-5 w-5 text-[var(--color-primary)]" />
        </div>
        <div className="min-w-0 flex-1">
          <h2 className="truncate text-2xl font-bold">{tag.name}</h2>
          <p className="text-sm text-[var(--color-muted-foreground)]">
            {tag.scene_count} scene{tag.scene_count === 1 ? "" : "s"}
            {parentName ? ` · under ${parentName}` : ""}
          </p>
        </div>
        <Button
          size="sm"
          variant="outline"
          onClick={() => navigate({ to: "/library/scenes", search: { tags: [tag.name] } })}
        >
          <Filter className="mr-1.5 h-3.5 w-3.5" />
          Filter scenes
        </Button>
      </div>

      <div className="flex flex-col gap-2 rounded-lg border border-[var(--color-border)] bg-[var(--color-card)] p-3 sm:flex-row sm:items-end">
        <label className="flex flex-1 flex-col gap-1 text-xs font-medium text-[var(--color-muted-foreground)]">
          Parent tag
          <select
            className="h-9 rounded-md border border-[var(--color-border)] bg-[var(--color-background)] px-2 text-sm text-[var(--color-foreground)]"
            value={parentEdit}
            onChange={(e) => setParentEdit(e.target.value)}
          >
            <option value="">None (top level)</option>
            {allTags
              .filter((t) => t.id !== tag.id)
              .map((t) => (
                <option key={t.id} value={t.id}>
                  {t.name}
                </option>
              ))}
          </select>
        </label>
        <Button
          size="sm"
          disabled={parentSaving}
          onClick={() => {
            setParentSaving(true);
            setParentError("");
            void api
              .setTagParent(tag.id, parentEdit || null)
              .then((updated) => {
                setTag(updated);
                setParentEdit(updated.parent_id ?? "");
                setAllTags((prev) => prev.map((t) => (t.id === updated.id ? updated : t)));
              })
              .catch((e) =>
                setParentError(e instanceof Error ? e.message : String(e) || "Save failed"),
              )
              .finally(() => setParentSaving(false));
          }}
        >
          {parentSaving ? "Saving…" : "Save parent"}
        </Button>
      </div>
      {parentError && <p className="text-xs text-red-400">{parentError}</p>}

      {children.length > 0 && (
        <div className="space-y-2">
          <p className="text-xs font-medium text-[var(--color-muted-foreground)]">Child tags</p>
          <div className="flex flex-wrap gap-2">
            {children.map((child) => (
              <Link
                key={child.id}
                to="/library/tags/$tagId"
                params={{ tagId: child.id }}
                className="rounded-full border border-[var(--color-border)] bg-[var(--color-card)] px-3 py-1 text-sm hover:border-[var(--color-primary)]"
              >
                {child.name}
                <span className="ml-1 text-[var(--color-muted-foreground)]">
                  {child.scene_count}
                </span>
              </Link>
            ))}
          </div>
        </div>
      )}

      {scenes.length === 0 ? (
        <EmptyState
          icon={<Tags className="h-12 w-12" />}
          title="No scenes found"
          description={`Nothing is tagged "${tag.name}" right now.`}
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
