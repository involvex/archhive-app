import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "@/lib/api/client";
import { sceneThumbUrl, isVideoScene } from "@/lib/mediaUrl";
import type { Scene, SceneFilter, SceneSort } from "@/lib/types";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { SceneEditDialog } from "@/components/SceneEditDialog";
import { SceneDetailsDialog } from "@/components/SceneDetailsDialog";
import { ScenePlayerDialog } from "@/components/ScenePlayerDialog";
import { SceneBulkEditBar } from "@/components/SceneBulkEditBar";
import { SceneContextMenu, type SceneContextMenuState } from "@/components/SceneContextMenu";
import { SceneCard } from "@/components/SceneCard";
import { SkeletonGrid } from "@/components/SkeletonGrid";
import { ErrorState } from "@/components/ErrorState";
import { EmptyState } from "@/components/EmptyState";
import { usePullToRefresh } from "@/lib/hooks/usePullToRefresh";
import { Film, LayoutGrid, List, RefreshCw, X } from "lucide-react";

export const Route = createFileRoute("/library/scenes/")({
  validateSearch: (search: Record<string, unknown>): { performers?: string[]; tags?: string[] } => {
    const parseArray = (key: string): string[] | undefined => {
      const v = search[key];
      if (Array.isArray(v)) return v.filter((x): x is string => typeof x === "string");
      if (typeof v === "string") return [v];
      return undefined;
    };
    return {
      performers: parseArray("performers"),
      tags: parseArray("tags"),
    };
  },
  component: ScenesPage,
});

const LONG_PRESS_MS = 480;

function FilterPill({ label, onRemove }: { label: string; onRemove: () => void }) {
  return (
    <span className="inline-flex items-center gap-1 rounded-full border border-[var(--color-border)] bg-[var(--color-card)] px-2 py-0.5 text-xs">
      {label}
      <button
        type="button"
        onClick={onRemove}
        className="hover:text-red-400"
        aria-label="Remove filter"
      >
        <X className="h-3 w-3" />
      </button>
    </span>
  );
}

function ScenesPage() {
  const navigate = useNavigate();
  const urlSearch = Route.useSearch();
  const [scenes, setScenes] = useState<Scene[]>([]);
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<SceneSort>("newest");
  const [filter, setFilter] = useState<SceneFilter>({});
  const [performerInput, setPerformerInput] = useState("");
  const [tagInput, setTagInput] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setFilter((prev) => {
      const next = { ...prev };
      let changed = false;
      const urlPerf = urlSearch.performers ?? [];
      const prevPerf = prev.performer_names ?? [];
      if (urlPerf.length !== prevPerf.length || !urlPerf.every((n, i) => n === prevPerf[i])) {
        next.performer_names = urlPerf;
        setPerformerInput(urlPerf.join(", "));
        changed = true;
      }
      const urlTagsArr = urlSearch.tags ?? [];
      const prevTags = prev.tag_names ?? [];
      if (urlTagsArr.length !== prevTags.length || !urlTagsArr.every((n, i) => n === prevTags[i])) {
        next.tag_names = urlTagsArr;
        setTagInput(urlTagsArr.join(", "));
        changed = true;
      }
      return changed ? next : prev;
    });
  }, [urlSearch.performers, urlSearch.tags]);
  const [editScene, setEditScene] = useState<Scene | null>(null);
  const [detailsScene, setDetailsScene] = useState<Scene | null>(null);
  const [playerScene, setPlayerScene] = useState<Scene | null>(null);
  const [playerIndex, setPlayerIndex] = useState(0);
  const [contextMenu, setContextMenu] = useState<SceneContextMenuState | null>(null);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [selectionMode, setSelectionMode] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<Scene | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [viewMode, setViewMode] = useState<"grid" | "list">("grid");
  const longPressTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const longPressTriggered = useRef(false);

  const hasFilter =
    filter.missing_thumb ||
    filter.missing_duration ||
    filter.hash_named ||
    filter.min_duration != null ||
    filter.max_duration != null ||
    (filter.performer_names?.length ?? 0) > 0 ||
    (filter.tag_names?.length ?? 0) > 0;

  const missingThumbCount = scenes.filter((s) => !s.thumb).length;
  const [genThumbsLoading, setGenThumbsLoading] = useState(false);
  const [genThumbsResult, setGenThumbsResult] = useState("");

  const refresh = useCallback(() => {
    setLoading(true);
    setError(null);
    const promise = hasFilter
      ? api.listScenesWithFilter(filter)
      : api.listScenes(query || undefined, sort);
    promise
      .then(setScenes)
      .catch((e) => {
        console.error(e);
        setError(e instanceof Error ? e.message : "Failed to load scenes");
      })
      .finally(() => setLoading(false));
  }, [query, sort, filter, hasFilter]);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    refresh();
  }, [refresh]);

  useEffect(() => {
    return () => {
      if (longPressTimer.current) clearTimeout(longPressTimer.current);
    };
  }, []);

  const handleRefresh = useCallback(async () => {
    refresh();
  }, [refresh]);

  const { containerRef, pullDistance, refreshing } = usePullToRefresh({
    onRefresh: handleRefresh,
    disabled: false,
  });

  function handleSaved(scene: Scene) {
    setScenes((prev) => prev.map((s) => (s.id === scene.id ? scene : s)));
    setPlayerScene((prev) => (prev?.id === scene.id ? scene : prev));
  }

  function handleDeleted(id: string) {
    setScenes((prev) => prev.filter((s) => s.id !== id));
    setPlayerScene((prev) => (prev?.id === id ? null : prev));
    setSelectedIds((prev) => {
      const next = new Set(prev);
      next.delete(id);
      return next;
    });
  }

  function openMenuAt(scene: Scene, x: number, y: number) {
    const maxX = typeof window !== "undefined" ? window.innerWidth - 200 : x;
    const maxY = typeof window !== "undefined" ? window.innerHeight - 260 : y;
    setContextMenu({
      scene,
      x: Math.max(8, Math.min(x, maxX)),
      y: Math.max(8, Math.min(y, maxY)),
    });
  }

  function clearLongPress() {
    if (longPressTimer.current) {
      clearTimeout(longPressTimer.current);
      longPressTimer.current = null;
    }
  }

  function onCardPointerDown(e: React.PointerEvent, scene: Scene) {
    if (selectionMode || e.pointerType === "mouse") return;
    longPressTriggered.current = false;
    clearLongPress();
    const { clientX, clientY } = e;
    longPressTimer.current = setTimeout(() => {
      longPressTriggered.current = true;
      openMenuAt(scene, clientX, clientY);
    }, LONG_PRESS_MS);
  }

  function onCardPointerUp() {
    clearLongPress();
  }

  function toggleSelect(id: string) {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  function selectAll() {
    setSelectedIds(new Set(scenes.map((s) => s.id)));
  }

  async function renameFileToTitle(scene: Scene) {
    try {
      const updated = await api.updateScene(scene.id, {
        title: scene.title,
        rename_file: true,
      });
      handleSaved(updated);
    } catch (e) {
      console.error(e);
    }
  }

  async function regenerateThumb(scene: Scene) {
    try {
      const updated = await api.probeSceneMetadata(scene.id);
      handleSaved(updated);
    } catch (e) {
      console.error(e);
    }
  }

  async function generateAllMissingThumbs() {
    setGenThumbsLoading(true);
    setGenThumbsResult("");
    try {
      const result = await api.generateMissingThumbs();
      setGenThumbsResult(
        `Generated ${result.generated} thumbnail${result.generated === 1 ? "" : "s"}`,
      );
      refresh();
    } catch (e) {
      setGenThumbsResult(e instanceof Error ? e.message : "Generation failed");
    } finally {
      setGenThumbsLoading(false);
    }
  }

  async function confirmDelete(deleteFiles: boolean) {
    if (!deleteTarget) return;
    setDeleting(true);
    try {
      await api.deleteScene(deleteTarget.id, deleteFiles);
      handleDeleted(deleteTarget.id);
      setDeleteTarget(null);
    } catch (e) {
      console.error(e);
    } finally {
      setDeleting(false);
    }
  }

  return (
    <div ref={containerRef} className="space-y-4">
      {pullDistance > 0 && (
        <div
          className="flex items-center justify-center transition-height overflow-hidden"
          style={{ height: pullDistance }}
        >
          <div
            className={`text-xs transition-opacity ${pullDistance > 60 ? "text-[var(--color-primary)]" : "text-[var(--color-muted-foreground)]"}`}
          >
            {refreshing
              ? "Refreshing..."
              : pullDistance > 60
                ? "Release to refresh"
                : "Pull to refresh"}
          </div>
        </div>
      )}

      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-2xl font-bold">Library — Scenes</h2>
        <div className="flex gap-2">
          <Button size="sm" variant="ghost" onClick={refresh} title="Refresh">
            <RefreshCw className="h-4 w-4" />
          </Button>
          {scenes.length > 0 && (
            <div className="flex rounded-md border border-[var(--color-border)] overflow-hidden">
              <button
                type="button"
                onClick={() => setViewMode("grid")}
                className={`p-1.5 transition-colors ${viewMode === "grid" ? "bg-[var(--color-muted)] text-[var(--color-foreground)]" : "text-[var(--color-muted-foreground)] hover:text-[var(--color-foreground)]"}`}
                aria-label="Grid view"
              >
                <LayoutGrid className="h-4 w-4" />
              </button>
              <button
                type="button"
                onClick={() => setViewMode("list")}
                className={`p-1.5 transition-colors ${viewMode === "list" ? "bg-[var(--color-muted)] text-[var(--color-foreground)]" : "text-[var(--color-muted-foreground)] hover:text-[var(--color-foreground)]"}`}
                aria-label="List view"
              >
                <List className="h-4 w-4" />
              </button>
            </div>
          )}
          <Button
            size="sm"
            variant={selectionMode ? "default" : "outline"}
            onClick={() => {
              setSelectionMode((v) => !v);
              setSelectedIds(new Set());
            }}
          >
            {selectionMode ? "Done selecting" : "Select multiple"}
          </Button>
          {selectionMode && (
            <Button size="sm" variant="outline" onClick={selectAll}>
              Select all
            </Button>
          )}
        </div>
      </div>

      <SceneBulkEditBar
        selectedIds={[...selectedIds]}
        onClear={() => setSelectedIds(new Set())}
        onApplied={refresh}
      />

      <div className="flex flex-wrap items-center gap-2">
        <Input
          placeholder="Search scenes..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          className="max-w-md"
        />
        <select
          className="h-9 rounded-md border border-[var(--color-border)] bg-[var(--color-background)] px-2 text-sm"
          value={sort}
          onChange={(e) => setSort(e.target.value as SceneSort)}
          aria-label="Sort scenes"
        >
          <option value="newest">Latest</option>
          <option value="downloaded">Recently Downloaded</option>
          <option value="name">Name</option>
        </select>
      </div>
      <div className="flex flex-wrap gap-1.5">
        {[
          { key: "all", label: "All" },
          { key: "missing_thumb", label: "Missing thumb" },
          { key: "missing_duration", label: "Missing duration" },
          { key: "short", label: "\u2264 15s" },
          { key: "hash_named", label: "Hash-named" },
        ].map(({ key, label }) => {
          const active =
            key === "all"
              ? !hasFilter
              : key === "short"
                ? filter.max_duration === 15
                : !!filter[key as keyof SceneFilter];
          return (
            <button
              key={key}
              type="button"
              onClick={() => {
                if (key === "all") setFilter({});
                else if (key === "short")
                  setFilter((f) => ({
                    ...f,
                    max_duration: f.max_duration === 15 ? undefined : 15,
                  }));
                else setFilter((f) => ({ ...f, [key]: !f[key as keyof SceneFilter] }));
              }}
              className={`rounded-full px-3 py-1 text-xs font-medium transition ${
                active
                  ? "bg-[var(--color-primary)] text-[var(--color-primary-foreground)]"
                  : "bg-[var(--color-secondary)] text-[var(--color-secondary-foreground)] hover:bg-[var(--color-muted)]"
              }`}
            >
              {label}
            </button>
          );
        })}
        <div className="flex items-center gap-1 ml-2">
          <Input
            type="number"
            placeholder="Min s"
            value={filter.min_duration ?? ""}
            onChange={(e) => {
              const v = e.target.value ? Number(e.target.value) : undefined;
              setFilter((f) => ({ ...f, min_duration: v }));
            }}
            className="h-7 w-16 text-xs"
            aria-label="Minimum duration in seconds"
          />
          <span className="text-xs text-[var(--color-muted-foreground)]">–</span>
          <Input
            type="number"
            placeholder="Max s"
            value={filter.max_duration ?? ""}
            onChange={(e) => {
              const v = e.target.value ? Number(e.target.value) : undefined;
              setFilter((f) => ({ ...f, max_duration: v }));
            }}
            className="h-7 w-16 text-xs"
            aria-label="Maximum duration in seconds"
          />
        </div>
      </div>

      {hasFilter && (
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-xs text-[var(--color-muted-foreground)]">Active filters:</span>
          {filter.missing_thumb && (
            <FilterPill
              label="Missing thumb"
              onRemove={() => setFilter((f) => ({ ...f, missing_thumb: false }))}
            />
          )}
          {filter.missing_duration && (
            <FilterPill
              label="Missing duration"
              onRemove={() => setFilter((f) => ({ ...f, missing_duration: false }))}
            />
          )}
          {filter.hash_named && (
            <FilterPill
              label="Hash-named"
              onRemove={() => setFilter((f) => ({ ...f, hash_named: false }))}
            />
          )}
          {filter.max_duration != null && (
            <FilterPill
              label={`≤ ${filter.max_duration}s`}
              onRemove={() => setFilter((f) => ({ ...f, max_duration: undefined }))}
            />
          )}
          {filter.min_duration != null && (
            <FilterPill
              label={`≥ ${filter.min_duration}s`}
              onRemove={() => setFilter((f) => ({ ...f, min_duration: undefined }))}
            />
          )}
          {(filter.performer_names?.length ?? 0) > 0 ? (
            filter.performer_names!.map((name) => (
              <FilterPill
                key={name}
                label={`Performer: ${name}`}
                onRemove={() => {
                  setFilter((f) => {
                    const names = (f.performer_names ?? []).filter((n) => n !== name);
                    return { ...f, performer_names: names.length > 0 ? names : undefined };
                  });
                  setPerformerInput(filter.performer_names!.filter((n) => n !== name).join(", "));
                  const remaining = filter.performer_names!.filter((n) => n !== name);
                  navigate({
                    to: "/library/scenes",
                    search: remaining.length > 0 ? { performers: remaining } : {},
                  });
                }}
              />
            ))
          ) : (
            <form
              onSubmit={(e) => {
                e.preventDefault();
                const v = performerInput.trim();
                if (v) {
                  const names = [...(filter.performer_names ?? []), v];
                  setFilter((f) => ({ ...f, performer_names: names }));
                  navigate({
                    to: "/library/scenes",
                    search: { performers: names },
                  });
                }
              }}
              className="flex items-center gap-1"
            >
              <Input
                placeholder="Filter performer…"
                value={performerInput}
                onChange={(e) => setPerformerInput(e.target.value)}
                className="h-7 w-36 text-xs"
              />
            </form>
          )}
          {(filter.tag_names?.length ?? 0) > 0 ? (
            filter.tag_names!.map((name) => (
              <FilterPill
                key={name}
                label={`Tag: ${name}`}
                onRemove={() => {
                  setFilter((f) => {
                    const names = (f.tag_names ?? []).filter((n) => n !== name);
                    return { ...f, tag_names: names.length > 0 ? names : undefined };
                  });
                  setTagInput(filter.tag_names!.filter((n) => n !== name).join(", "));
                  const remaining = filter.tag_names!.filter((n) => n !== name);
                  navigate({
                    to: "/library/scenes",
                    search: remaining.length > 0 ? { tags: remaining } : {},
                  });
                }}
              />
            ))
          ) : (
            <form
              onSubmit={(e) => {
                e.preventDefault();
                const v = tagInput.trim();
                if (v) {
                  const names = [...(filter.tag_names ?? []), v];
                  setFilter((f) => ({ ...f, tag_names: names }));
                  navigate({
                    to: "/library/scenes",
                    search: { tags: names },
                  });
                }
              }}
              className="flex items-center gap-1"
            >
              <Input
                placeholder="Filter tag…"
                value={tagInput}
                onChange={(e) => setTagInput(e.target.value)}
                className="h-7 w-36 text-xs"
              />
            </form>
          )}
          <button
            type="button"
            onClick={() => {
              setFilter({});
              setPerformerInput("");
              setTagInput("");
              navigate({ to: "/library/scenes", search: {} });
            }}
            className="text-xs text-[var(--color-muted-foreground)] underline hover:text-[var(--color-foreground)]"
          >
            Clear all
          </button>
        </div>
      )}
      {genThumbsResult && (
        <p className="rounded-md border border-green-400/30 bg-green-400/10 px-3 py-2 text-sm text-green-400">
          {genThumbsResult}
        </p>
      )}
      {missingThumbCount > 0 && (
        <div className="flex flex-wrap items-center gap-3 rounded-md border border-amber-400/30 bg-amber-400/10 px-3 py-2">
          <p className="text-sm text-amber-400">
            {missingThumbCount} scene{missingThumbCount === 1 ? "" : "s"} missing thumbnail
            {missingThumbCount === 1 ? "" : "s"}.
          </p>
          <Button
            size="sm"
            variant="outline"
            onClick={() => void generateAllMissingThumbs()}
            disabled={genThumbsLoading}
          >
            {genThumbsLoading ? "Generating…" : "Generate now"}
          </Button>
        </div>
      )}
      {loading ? (
        <SkeletonGrid count={12} cols={3} />
      ) : error ? (
        <ErrorState message={error} onRetry={refresh} />
      ) : scenes.length === 0 ? (
        <EmptyState
          icon={<Film className="h-12 w-12" />}
          title="No scenes found"
          description="Try adjusting your search or filter criteria."
        />
      ) : viewMode === "grid" ? (
        <div className="grid grid-cols-3 sm:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
          {scenes.map((scene) => {
            const thumbSrc = sceneThumbUrl(scene);
            const isSelected = selectedIds.has(scene.id);
            const canPlay = isVideoScene(scene);
            return (
              <button
                key={scene.id}
                type="button"
                className="w-full text-left [appearance:button] bg-transparent border-0 p-0"
                onPointerDown={(e) => onCardPointerDown(e, scene)}
                onPointerUp={onCardPointerUp}
                onPointerCancel={onCardPointerUp}
                onPointerLeave={onCardPointerUp}
                onClick={() => {
                  if (!selectionMode) return;
                  toggleSelect(scene.id);
                }}
              >
                <SceneCard
                  item={scene}
                  thumbSrc={thumbSrc}
                  selected={isSelected}
                  selectionMode={selectionMode}
                  onEdit={setEditScene}
                  onContextMenu={(item, x, y) => openMenuAt(item as Scene, x, y)}
                  onClick={(item) => {
                    const s = item as Scene;
                    if (longPressTriggered.current) {
                      longPressTriggered.current = false;
                      return;
                    }
                    if (canPlay) {
                      const videoScenes = scenes.filter(isVideoScene);
                      const idx = videoScenes.findIndex((v) => v.id === s.id);
                      if (idx >= 0) setPlayerIndex(idx);
                      setPlayerScene(s);
                    } else {
                      setDetailsScene(s);
                    }
                  }}
                />
              </button>
            );
          })}
        </div>
      ) : (
        <div className="flex flex-col gap-2">
          {scenes.map((scene) => {
            const thumbSrc = sceneThumbUrl(scene);
            const isSelected = selectedIds.has(scene.id);
            const canPlay = isVideoScene(scene);
            return (
              <button
                key={scene.id}
                type="button"
                className="w-full text-left [appearance:button] bg-transparent border-0 p-0"
                onPointerDown={(e) => onCardPointerDown(e, scene)}
                onPointerUp={onCardPointerUp}
                onPointerCancel={onCardPointerUp}
                onPointerLeave={onCardPointerUp}
                onClick={() => {
                  if (!selectionMode) return;
                  toggleSelect(scene.id);
                }}
              >
                <SceneCard
                  item={scene}
                  thumbSrc={thumbSrc}
                  selected={isSelected}
                  selectionMode={selectionMode}
                  listView
                  onEdit={setEditScene}
                  onContextMenu={(item, x, y) => openMenuAt(item as Scene, x, y)}
                  onClick={(item) => {
                    const s = item as Scene;
                    if (longPressTriggered.current) {
                      longPressTriggered.current = false;
                      return;
                    }
                    if (canPlay) {
                      const videoScenes = scenes.filter(isVideoScene);
                      const idx = videoScenes.findIndex((v) => v.id === s.id);
                      if (idx >= 0) setPlayerIndex(idx);
                      setPlayerScene(s);
                    } else {
                      setDetailsScene(s);
                    }
                  }}
                />
              </button>
            );
          })}
        </div>
      )}

      <SceneEditDialog
        scene={editScene}
        open={editScene !== null}
        onClose={() => setEditScene(null)}
        onSaved={handleSaved}
        onDeleted={handleDeleted}
      />

      <SceneDetailsDialog
        scene={detailsScene}
        open={detailsScene !== null}
        onClose={() => setDetailsScene(null)}
      />

      <ScenePlayerDialog
        scene={playerScene}
        scenes={scenes.filter(isVideoScene)}
        currentIndex={playerIndex}
        open={playerScene !== null}
        onClose={() => setPlayerScene(null)}
        onEdit={(s) => {
          setPlayerScene(null);
          setEditScene(s);
        }}
        onNavigate={(s, i) => {
          setPlayerScene(s);
          setPlayerIndex(i);
        }}
      />

      <SceneContextMenu
        menu={contextMenu}
        onClose={() => setContextMenu(null)}
        onEdit={(s) => setEditScene(s)}
        onDetails={(s) => setDetailsScene(s)}
        onPlay={(s) => {
          const videoScenes = scenes.filter(isVideoScene);
          const idx = videoScenes.findIndex((v) => v.id === s.id);
          if (idx >= 0) setPlayerIndex(idx);
          setPlayerScene(s);
        }}
        onOpenExplorer={(s) => void api.openSceneInExplorer(s.id).catch(console.error)}
        onOpenDefault={(s) => void api.openSceneWithDefault(s.id).catch(console.error)}
        onRenameFile={(s) => void renameFileToTitle(s)}
        onRegenThumb={(s) => void regenerateThumb(s)}
        onDelete={(s) => setDeleteTarget(s)}
      />

      {deleteTarget && (
        <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 p-4">
          <div
            role="dialog"
            aria-modal="true"
            className="w-full max-w-md rounded-lg border border-[var(--color-border)] bg-[var(--color-card)] p-4 shadow-xl"
          >
            <h3 className="text-lg font-semibold">Delete scene</h3>
            <p className="mt-2 text-sm text-[var(--color-muted-foreground)]">
              Remove "{deleteTarget.title}" from the library?
            </p>
            <div className="mt-4 flex flex-col gap-2 sm:flex-row sm:justify-end">
              <Button variant="outline" onClick={() => setDeleteTarget(null)} disabled={deleting}>
                Cancel
              </Button>
              <Button
                variant="outline"
                onClick={() => void confirmDelete(false)}
                disabled={deleting}
              >
                Remove from library
              </Button>
              <Button
                variant="destructive"
                onClick={() => void confirmDelete(true)}
                disabled={deleting}
              >
                Also delete file
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
