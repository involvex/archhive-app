import { createFileRoute, useSearch } from "@tanstack/react-router";
import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "@/lib/api/client";
import type { Collection, CollectionType, Scene } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { SceneCard } from "@/components/SceneCard";
import { SkeletonGrid } from "@/components/SkeletonGrid";
import { Skeleton } from "@/components/ui/skeleton";
import { EmptyState } from "@/components/EmptyState";
import { sceneThumbUrl, isVideoScene } from "@/lib/mediaUrl";
import { ScenePlayerDialog } from "@/components/ScenePlayerDialog";
import { SceneDetailsDialog } from "@/components/SceneDetailsDialog";
import { SceneContextMenu, type SceneContextMenuState } from "@/components/SceneContextMenu";
import { SceneEditDialog } from "@/components/SceneEditDialog";
import { CollectionPickerDialog } from "@/components/CollectionPickerDialog";
import { CollectionFormDialog } from "@/components/CollectionFormDialog";
import {
  Plus,
  Folder,
  List,
  LayoutGrid,
  FolderOpen,
  X,
  FolderSymlink,
  Download,
  GripVertical,
} from "lucide-react";

export const Route = createFileRoute("/library/collections/")({
  validateSearch: (search: Record<string, unknown>): { newSmartFilter?: string } => {
    return {
      newSmartFilter: typeof search.newSmartFilter === "string" ? search.newSmartFilter : undefined,
    };
  },
  component: CollectionsPage,
});

function coverSrc(cover: { scene_id: string; thumb?: string }): string | undefined {
  return sceneThumbUrl({
    id: cover.scene_id,
    title: "",
    thumb: cover.thumb,
    performers: [],
    tags: [],
    created_at: "",
  } as Scene);
}

function CollectionsPage() {
  const search = useSearch({ strict: false });
  const [collections, setCollections] = useState<Collection[]>([]);
  const [selected, setSelected] = useState<Collection | null>(null);
  const [scenes, setScenes] = useState<Scene[]>([]);
  const [loading, setLoading] = useState(true);
  const [scenesLoading, setScenesLoading] = useState(false);
  const [showCreate, setShowCreate] = useState(false);
  const [pickerOpen, setPickerOpen] = useState<Scene | null>(null);
  const [editScene, setEditScene] = useState<Scene | null>(null);
  const [detailsScene, setDetailsScene] = useState<Scene | null>(null);
  const [playerScene, setPlayerScene] = useState<Scene | null>(null);
  const [playerIndex, setPlayerIndex] = useState(0);
  const [contextMenu, setContextMenu] = useState<SceneContextMenuState | null>(null);
  const [viewMode, setViewMode] = useState<"grid" | "list">("grid");
  const [opError, setOpError] = useState("");
  const [dragId, setDragId] = useState<string | null>(null);

  const processedFilterRef = useRef<string | null>(null);
  const canReorder =
    selected != null && selected.type !== "smart" && scenes.length > 1 && viewMode === "list";

  const loadCollections = useCallback(() => {
    setLoading(true);
    api
      .listCollections()
      .then(setCollections)
      .catch(() => setCollections([]))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    loadCollections();
  }, [loadCollections]);

  // Handle pre-filled Smart Collection filter from URL
  useEffect(() => {
    const filterParam = search.newSmartFilter;
    if (filterParam && filterParam !== processedFilterRef.current) {
      processedFilterRef.current = filterParam;
      try {
        const filter = JSON.parse(decodeURIComponent(filterParam));
        if (Object.keys(filter).length > 0) {
          // eslint-disable-next-line react-hooks/set-state-in-effect
          setShowCreate(true);
        }
      } catch (e) {
        console.error("Failed to parse smart filter:", e);
      }
    }
  }, [search.newSmartFilter]);

  const loadScenes = (col: Collection) => {
    setSelected(col);
    setScenesLoading(true);
    api
      .listCollectionScenes(col.id)
      .then(setScenes)
      .catch(() => setScenes([]))
      .finally(() => setScenesLoading(false));
  };

  const handleCreate = (data: {
    name: string;
    type: CollectionType;
    description?: string;
    filter?: import("@/lib/types").SceneFilter;
  }) => {
    setOpError("");
    api
      .createCollection({
        name: data.name,
        type: data.type,
        description: data.description,
        filter: data.filter,
      })
      .then(() => loadCollections())
      .catch((e) => {
        console.error(e);
        setOpError(e instanceof Error ? e.message : "Failed to create collection");
      });
  };

  const handleDelete = (id: string) => {
    if (!confirm("Delete this collection? Scenes will not be removed.")) return;
    setOpError("");
    api
      .deleteCollection(id)
      .then(() => {
        loadCollections();
        if (selected?.id === id) setSelected(null);
      })
      .catch((e) => {
        console.error(e);
        setOpError(e instanceof Error ? e.message : "Failed to delete collection");
      });
  };

  const handleAddToCollection = (scene: Scene) => {
    setPickerOpen(scene);
  };

  const handleRemoveFromCollection = async (scene: Scene) => {
    if (!selected || selected.type === "smart") return;
    setOpError("");
    try {
      await api.removeSceneFromCollection(scene.id, selected.id);
      setScenes((prev) => prev.filter((s) => s.id !== scene.id));
      loadCollections();
    } catch (e) {
      setOpError(e instanceof Error ? e.message : "Failed to remove from collection");
    }
  };

  const handlePickerChange = async (collectionId: string | null, scene: Scene) => {
    try {
      const existing = await api.sceneCollectionIds(scene.id);
      if (collectionId) {
        if (existing.includes(collectionId)) {
          await api.removeSceneFromCollection(scene.id, collectionId);
        } else {
          await api.addSceneToCollection(scene.id, collectionId);
        }
        if (selected && !existing.includes(collectionId)) {
          const updated = await api.listCollectionScenes(selected.id);
          setScenes(updated);
        }
        loadCollections();
      }
    } catch (e) {
      console.error(e);
    }
  };

  const persistOrder = async (next: Scene[]) => {
    if (!selected || selected.type === "smart") return;
    setScenes(next);
    try {
      await api.reorderCollectionScenes(
        selected.id,
        next.map((s) => s.id),
      );
    } catch (e) {
      setOpError(e instanceof Error ? e.message : "Failed to reorder");
      loadScenes(selected);
    }
  };

  const onDragStart = (id: string) => setDragId(id);
  const onDragOver = (e: React.DragEvent, overId: string) => {
    e.preventDefault();
    if (!dragId || dragId === overId) return;
    const from = scenes.findIndex((s) => s.id === dragId);
    const to = scenes.findIndex((s) => s.id === overId);
    if (from < 0 || to < 0) return;
    const next = [...scenes];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    setScenes(next);
  };
  const onDragEnd = () => {
    if (dragId) void persistOrder(scenes);
    setDragId(null);
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold">Collections and Watchlists</h2>
        <Button size="sm" onClick={() => setShowCreate(true)}>
          <Plus className="h-4 w-4 mr-1" />
          New
        </Button>
      </div>

      {opError && (
        <p className="rounded-md border border-red-400/30 bg-red-400/10 px-3 py-2 text-sm text-red-400">
          {opError}
        </p>
      )}

      {loading ? (
        <div className="space-y-2">
          {Array.from({ length: 4 }, (_, i) => i + 1).map((n) => (
            <Skeleton key={n} className="h-16 w-full rounded-md" />
          ))}
        </div>
      ) : collections.length === 0 ? (
        <EmptyState
          icon={<FolderOpen className="h-12 w-12" />}
          title="No collections yet"
          description="Create a collection to group scenes together."
          action={
            <Button onClick={() => setShowCreate(true)}>
              <Plus className="h-4 w-4 mr-1" />
              Create collection
            </Button>
          }
        />
      ) : (
        <div className="space-y-2">
          {collections.map((col) => {
            const isSmart = col.type === "smart";
            const covers = (col.cover_thumbs ?? []).slice(0, 4);
            return (
              <div
                key={col.id}
                className={`flex items-center justify-between rounded-md border p-3 transition-colors ${
                  selected?.id === col.id
                    ? "border-[var(--color-primary)] bg-[var(--color-accent)]/10"
                    : "border-[var(--color-border)] hover:bg-[var(--color-muted)]/30"
                }`}
              >
                <button
                  type="button"
                  className="flex items-center gap-2 text-left flex-1"
                  onClick={() => loadScenes(col)}
                >
                  {covers.length > 0 ? (
                    <div className="relative w-12 h-12 rounded-md overflow-hidden mr-1 flex-shrink-0 bg-[var(--color-muted)]">
                      <div className="grid grid-cols-2 gap-px w-full h-full">
                        {covers.map((c) => (
                          <img
                            key={c.scene_id}
                            src={coverSrc(c) || "/placeholder.png"}
                            alt=""
                            className="w-full h-full object-cover"
                          />
                        ))}
                      </div>
                    </div>
                  ) : (
                    <Folder className="h-4 w-4" />
                  )}
                  <div>
                    <span className="font-medium">{col.name}</span>
                    <span className="text-xs text-[var(--color-muted-foreground)] ml-2">
                      {col.type === "watchlist" ? "Watchlist" : isSmart ? "Smart" : "Collection"} -{" "}
                      {col.scene_count} scene{col.scene_count !== 1 ? "s" : ""}
                    </span>
                  </div>
                </button>
                <button
                  type="button"
                  onClick={() => handleDelete(col.id)}
                  className="p-1 text-red-400 hover:text-red-300"
                  aria-label="Delete collection"
                >
                  <X className="h-3 w-3" />
                </button>
              </div>
            );
          })}
        </div>
      )}

      {selected && (
        <div className="space-y-3 pt-4 border-t border-[var(--color-border)]">
          <div className="flex items-center justify-between">
            <h3 className="text-lg font-semibold">{selected.name} - Scenes</h3>
            <div className="flex gap-1">
              {canReorder && (
                <span className="mr-2 self-center text-xs text-[var(--color-muted-foreground)]">
                  Drag to reorder
                </span>
              )}
              <button
                type="button"
                onClick={async () => {
                  try {
                    const { content, filename } = await api.exportCollection(selected.id, "m3u");
                    const blob = new Blob([content], { type: "audio/x-mpegurl" });
                    const url = URL.createObjectURL(blob);
                    const a = document.createElement("a");
                    a.href = url;
                    a.download = filename;
                    a.click();
                    URL.revokeObjectURL(url);
                  } catch (e) {
                    console.error("Export failed:", e);
                  }
                }}
                className="p-1 text-muted-foreground"
                aria-label="Export as M3U"
              >
                <Download className="h-4 w-4" />
              </button>
              <button
                type="button"
                onClick={() => setViewMode("grid")}
                className={`p-1 ${viewMode === "grid" ? "text-primary" : "text-muted-foreground"}`}
                aria-label="Grid view"
              >
                <LayoutGrid className="h-4 w-4" />
              </button>
              <button
                type="button"
                onClick={() => setViewMode("list")}
                className={`p-1 ${viewMode === "list" ? "text-primary" : "text-muted-foreground"}`}
                aria-label="List view"
              >
                <List className="h-4 w-4" />
              </button>
            </div>
          </div>

          {scenesLoading ? (
            <SkeletonGrid count={8} cols={3} />
          ) : scenes.length === 0 ? (
            <EmptyState
              icon={<FolderSymlink className="h-8 w-8" />}
              title="No scenes in this collection"
              description="Add scenes from the scene context menu."
            />
          ) : viewMode === "grid" ? (
            <div className="grid grid-cols-3 sm:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
              {scenes.map((scene) => {
                const thumbSrc = sceneThumbUrl(scene);
                const canPlay = isVideoScene(scene);
                return (
                  <button
                    key={scene.id}
                    type="button"
                    className="w-full text-left [appearance:button] bg-transparent border-0 p-0"
                    onClick={() => {
                      if (canPlay) {
                        const idx = scenes.filter(isVideoScene).findIndex((v) => v.id === scene.id);
                        setPlayerIndex(idx >= 0 ? idx : 0);
                        setPlayerScene(scene);
                      } else {
                        setDetailsScene(scene);
                      }
                    }}
                  >
                    <SceneCard
                      item={scene}
                      thumbSrc={thumbSrc}
                      selected={false}
                      selectionMode={false}
                      watch={null}
                      onEdit={setEditScene}
                      onContextMenu={(item, x, y) => setContextMenu({ scene: item as Scene, x, y })}
                      onClick={() => {}}
                    />
                  </button>
                );
              })}
            </div>
          ) : (
            <div className="flex flex-col gap-2">
              {scenes.map((scene) => {
                const canPlay = isVideoScene(scene);
                return (
                  <div
                    key={scene.id}
                    draggable={canReorder}
                    onDragStart={() => onDragStart(scene.id)}
                    onDragOver={(e) => onDragOver(e, scene.id)}
                    onDragEnd={onDragEnd}
                    className={`flex items-center gap-2 rounded-md border border-[var(--color-border)] p-2 ${
                      dragId === scene.id ? "opacity-60" : ""
                    }`}
                  >
                    {canReorder && (
                      <GripVertical
                        className="h-4 w-4 shrink-0 cursor-grab text-[var(--color-muted-foreground)]"
                        aria-hidden
                      />
                    )}
                    <button
                      type="button"
                      className="flex flex-1 items-center gap-3 text-left hover:bg-[var(--color-muted)]/30 rounded-md"
                      onClick={() => {
                        if (canPlay) {
                          const idx = scenes
                            .filter(isVideoScene)
                            .findIndex((v) => v.id === scene.id);
                          setPlayerIndex(idx >= 0 ? idx : 0);
                          setPlayerScene(scene);
                        } else {
                          setDetailsScene(scene);
                        }
                      }}
                      onContextMenu={(e) => {
                        e.preventDefault();
                        setContextMenu({ scene, x: e.clientX, y: e.clientY });
                      }}
                    >
                      <img
                        src={sceneThumbUrl(scene) || "/placeholder.png"}
                        alt={scene.title}
                        className="h-12 w-16 object-cover rounded"
                      />
                      <div className="flex-1 min-w-0">
                        <div className="font-medium truncate">{scene.title}</div>
                        <div className="text-xs text-[var(--color-muted-foreground)]">
                          {scene.duration
                            ? `${Math.floor(scene.duration / 60)}:${String(scene.duration % 60).padStart(2, "0")}`
                            : "\u2014"}
                        </div>
                      </div>
                    </button>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      )}

      <CollectionFormDialog
        open={showCreate}
        onClose={() => setShowCreate(false)}
        onSubmit={handleCreate}
      />

      <CollectionPickerDialog
        scene={pickerOpen}
        open={pickerOpen !== null}
        onClose={() => setPickerOpen(null)}
        collections={collections}
        onCollectionChange={handlePickerChange}
      />

      <SceneEditDialog
        scene={editScene}
        open={editScene !== null}
        onClose={() => setEditScene(null)}
        onSaved={() => {
          if (selected) loadScenes(selected);
        }}
        onDeleted={() => {
          if (selected) loadScenes(selected);
        }}
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
        onEdit={setEditScene}
        onDetails={setDetailsScene}
        onPlay={(s) => {
          const idx = scenes.filter(isVideoScene).findIndex((v) => v.id === s.id);
          setPlayerIndex(idx >= 0 ? idx : 0);
          setPlayerScene(s);
        }}
        onAddToCollection={handleAddToCollection}
        onRemoveFromCollection={
          selected && selected.type !== "smart" ? handleRemoveFromCollection : undefined
        }
      />
    </div>
  );
}
