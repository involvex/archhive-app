import { useCallback, useState } from "react";
import type { DuplicateGroup, Scene } from "@/lib/types";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { SceneCard } from "@/components/SceneCard";
import { GitMerge, Trash2 } from "lucide-react";

interface DuplicateGroupCardProps {
  group: DuplicateGroup;
  onMerge: (keepId: string, removeIds: string[], deleteFiles: boolean) => Promise<void>;
  onDelete: (sceneId: string) => Promise<void>;
  onNavigate?: (scene: Scene) => void;
}

export function DuplicateGroupCard({
  group,
  onMerge,
  onDelete,
  onNavigate,
}: DuplicateGroupCardProps) {
  const [keepId, setKeepId] = useState<string>(group.scenes[0]?.id ?? "");
  const [deleteFiles, setDeleteFiles] = useState(false);
  const [merging, setMerging] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const removeIds = group.scenes.filter((s) => s.id !== keepId).map((s) => s.id);
  const canMerge = keepId && removeIds.length > 0;

  const handleMerge = useCallback(async () => {
    if (!canMerge) return;
    setMerging(true);
    try {
      await onMerge(keepId, removeIds, deleteFiles);
    } finally {
      setMerging(false);
    }
  }, [canMerge, keepId, removeIds, deleteFiles, onMerge]);

  const handleDelete = useCallback(
    async (sceneId: string) => {
      setDeletingId(sceneId);
      try {
        await onDelete(sceneId);
      } finally {
        setDeletingId(null);
      }
    },
    [onDelete],
  );

  return (
    <Card>
      <CardContent className="p-4 space-y-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2 text-sm">
            <span
              className={`rounded px-2 py-0.5 text-xs font-medium ${
                group.match_type === "phash"
                  ? "bg-blue-500/10 text-blue-400"
                  : "bg-green-500/10 text-green-400"
              }`}
            >
              {group.match_type === "phash" ? "Perceptual" : "Exact file"}
            </span>
            <span className="text-xs text-[var(--color-muted-foreground)]">
              {group.match_type === "phash"
                ? `pHash distance ≤ ${group.max_distance ?? "?"}`
                : "oShash match"}
            </span>
            <code className="text-[10px] text-[var(--color-muted-foreground)]">{group.hash}</code>
          </div>
          <div className="flex items-center gap-2">
            <label className="flex items-center gap-1.5 text-xs text-[var(--color-muted-foreground)]">
              <input
                type="checkbox"
                checked={deleteFiles}
                onChange={(e) => setDeleteFiles(e.target.checked)}
              />
              Delete files
            </label>
            <Button
              size="sm"
              variant="destructive"
              onClick={handleMerge}
              disabled={!canMerge || merging}
            >
              <GitMerge className="h-3.5 w-3.5 mr-1" />
              {merging ? "Merging…" : `Keep 1, remove ${removeIds.length}`}
            </Button>
          </div>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
          {group.scenes.map((scene) => (
            <div key={scene.id} className="relative">
              <button
                type="button"
                onClick={() => setKeepId(scene.id)}
                className={`absolute top-2 left-2 z-10 h-5 w-5 rounded-full border-2 flex items-center justify-center transition-colors ${
                  keepId === scene.id
                    ? "bg-[var(--color-primary)] border-[var(--color-primary)]"
                    : "bg-white/90 dark:bg-black/80 border-[var(--color-border)]"
                }`}
                title={keepId === scene.id ? "Keeping this scene" : "Click to keep this scene"}
              >
                {keepId === scene.id && (
                  <svg
                    className="h-3 w-3 text-white"
                    viewBox="0 0 12 12"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                  >
                    <path d="M2 6l3 3 5-5" />
                  </svg>
                )}
              </button>
              <SceneCard item={scene} listView={false} onClick={() => onNavigate?.(scene)} />
              <Button
                size="icon"
                variant="ghost"
                className="absolute top-2 right-2 h-7 w-7 bg-white/90 dark:bg-black/80 text-red-400 hover:text-red-300"
                onClick={(e) => {
                  e.stopPropagation();
                  void handleDelete(scene.id);
                }}
                disabled={deletingId === scene.id}
                title="Remove from library"
              >
                <Trash2 className="h-3.5 w-3.5" />
              </Button>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
