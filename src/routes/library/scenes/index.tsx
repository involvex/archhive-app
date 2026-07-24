import { createFileRoute } from "@tanstack/react-router";
import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "@/lib/api/client";
import { sceneThumbUrl, isVideoScene } from "@/lib/mediaUrl";
import type { Scene, SceneSort } from "@/lib/types";
import { Input } from "@/components/ui/input";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { SceneEditDialog } from "@/components/SceneEditDialog";
import { SceneDetailsDialog } from "@/components/SceneDetailsDialog";
import { ScenePlayerDialog } from "@/components/ScenePlayerDialog";
import { SceneBulkEditBar } from "@/components/SceneBulkEditBar";
import { SceneContextMenu, type SceneContextMenuState } from "@/components/SceneContextMenu";
import { MoreVertical, Pencil } from "lucide-react";

export const Route = createFileRoute("/library/scenes/")({
  component: ScenesPage,
});

const LONG_PRESS_MS = 480;

function ScenesPage() {
  const [scenes, setScenes] = useState<Scene[]>([]);
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<SceneSort>("newest");
  const [editScene, setEditScene] = useState<Scene | null>(null);
  const [detailsScene, setDetailsScene] = useState<Scene | null>(null);
  const [playerScene, setPlayerScene] = useState<Scene | null>(null);
  const [contextMenu, setContextMenu] = useState<SceneContextMenuState | null>(null);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [selectionMode, setSelectionMode] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<Scene | null>(null);
  const [deleting, setDeleting] = useState(false);
  const longPressTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const longPressTriggered = useRef(false);

  const refresh = useCallback(() => {
    void api
      .listScenes(query || undefined, sort)
      .then(setScenes)
      .catch(console.error);
  }, [query, sort]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  useEffect(() => {
    return () => {
      if (longPressTimer.current) clearTimeout(longPressTimer.current);
    };
  }, []);

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

  function handleContextMenu(e: React.MouseEvent, scene: Scene) {
    e.preventDefault();
    openMenuAt(scene, e.clientX, e.clientY);
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
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-2xl font-bold">Library — Scenes</h2>
        <div className="flex gap-2">
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
          <option value="name">Name</option>
        </select>
      </div>
      <div className="grid grid-cols-2 gap-3 md:grid-cols-4 lg:grid-cols-5">
        {scenes.map((scene) => {
          const thumbSrc = sceneThumbUrl(scene);
          const isSelected = selectedIds.has(scene.id);
          return (
            <Card
              key={scene.id}
              className={`overflow-hidden group cursor-pointer ${isSelected ? "ring-2 ring-[var(--color-primary)]" : ""}`}
              onContextMenu={(e) => handleContextMenu(e, scene)}
              onPointerDown={(e) => onCardPointerDown(e, scene)}
              onPointerUp={onCardPointerUp}
              onPointerCancel={onCardPointerUp}
              onPointerLeave={onCardPointerUp}
              onClick={() => {
                if (selectionMode) return;
                if (longPressTriggered.current) {
                  longPressTriggered.current = false;
                  return;
                }
                if (isVideoScene(scene)) setPlayerScene(scene);
                else setDetailsScene(scene);
              }}
            >
              <div className="aspect-video bg-[var(--color-muted)] relative">
                {selectionMode && (
                  <label className="absolute top-1 left-1 z-10 flex h-8 w-8 items-center justify-center rounded bg-black/60">
                    <input
                      type="checkbox"
                      checked={isSelected}
                      onChange={() => toggleSelect(scene.id)}
                      className="h-4 w-4"
                    />
                  </label>
                )}
                {thumbSrc ? (
                  <img
                    src={thumbSrc}
                    alt={scene.title}
                    className="h-full w-full object-cover"
                    onError={(e) => {
                      e.currentTarget.style.display = "none";
                    }}
                  />
                ) : (
                  <div className="flex h-full items-center justify-center text-[var(--color-muted-foreground)] text-xs">
                    No preview
                  </div>
                )}
                {!selectionMode && (
                  <div className="absolute top-1 right-1 z-10 flex gap-1">
                    <Button
                      size="sm"
                      variant="secondary"
                      className="h-8 w-8 p-0 bg-black/65 text-white hover:bg-black/80"
                      onClick={(e) => {
                        e.stopPropagation();
                        setEditScene(scene);
                      }}
                      aria-label="Edit scene"
                    >
                      <Pencil className="h-3.5 w-3.5" />
                    </Button>
                    <Button
                      size="sm"
                      variant="secondary"
                      className="h-8 w-8 p-0 bg-black/65 text-white hover:bg-black/80"
                      onClick={(e) => {
                        e.stopPropagation();
                        const rect = e.currentTarget.getBoundingClientRect();
                        openMenuAt(scene, rect.left, rect.bottom + 4);
                      }}
                      aria-label="More actions"
                    >
                      <MoreVertical className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                )}
              </div>
              <CardContent className="p-2 space-y-1">
                <p className="line-clamp-2 text-xs font-medium">{scene.title}</p>
                {scene.performers.length > 0 && (
                  <p className="text-[10px] text-[var(--color-muted-foreground)] truncate">
                    {scene.performers.join(", ")}
                  </p>
                )}
                <div className="flex flex-wrap gap-1">
                  {scene.tags.slice(0, 3).map((tag) => (
                    <span
                      key={tag}
                      className="rounded bg-[var(--color-secondary)] px-1.5 py-0.5 text-[10px]"
                    >
                      {tag}
                    </span>
                  ))}
                </div>
              </CardContent>
            </Card>
          );
        })}
      </div>
      {scenes.length === 0 && (
        <p className="text-sm text-[var(--color-muted-foreground)]">No scenes in library.</p>
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
        open={playerScene !== null}
        onClose={() => setPlayerScene(null)}
        onEdit={(s) => {
          setPlayerScene(null);
          setEditScene(s);
        }}
      />

      <SceneContextMenu
        menu={contextMenu}
        onClose={() => setContextMenu(null)}
        onEdit={(s) => setEditScene(s)}
        onDetails={(s) => setDetailsScene(s)}
        onPlay={(s) => setPlayerScene(s)}
        onOpenExplorer={(s) => void api.openSceneInExplorer(s.id).catch(console.error)}
        onOpenDefault={(s) => void api.openSceneWithDefault(s.id).catch(console.error)}
        onRenameFile={(s) => void renameFileToTitle(s)}
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
              Remove “{deleteTarget.title}” from the library?
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
