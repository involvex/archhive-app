import { useState, useEffect } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { api } from "@/lib/api/client";
import type { Collection, Scene } from "@/lib/types";
import { FolderSymlink, ListTodo } from "lucide-react";

interface CollectionPickerDialogProps {
  /** Single-scene mode (toggle membership). */
  scene?: Scene | null;
  /** Bulk mode: add all these scene ids to the chosen collection. */
  sceneIds?: string[];
  open: boolean;
  onClose: () => void;
  collections: Collection[];
  onCollectionChange?: (collectionId: string | null, scene: Scene) => void;
  /** Called after bulk add completes successfully. */
  onBulkAdded?: (collectionId: string, count: number) => void;
}

export function CollectionPickerDialog({
  scene = null,
  sceneIds,
  open,
  onClose,
  collections,
  onCollectionChange,
  onBulkAdded,
}: CollectionPickerDialogProps) {
  const [membership, setMembership] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const bulk = Boolean(sceneIds && sceneIds.length > 0 && !scene);

  useEffect(() => {
    if (!scene || bulk) return;
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setLoading(true);
    setError(null);
    api
      .sceneCollectionIds(scene.id)
      .then((ids) => setMembership(new Set(ids)))
      .catch(() => setMembership(new Set()))
      .finally(() => setLoading(false));
  }, [scene, bulk]);

  if (!scene && !bulk) return null;

  const toggle = (colId: string) => {
    if (!scene || !onCollectionChange) return;
    const wasMember = membership.has(colId);
    onCollectionChange(colId, scene);
    if (wasMember) {
      setMembership((prev) => {
        const next = new Set(prev);
        next.delete(colId);
        return next;
      });
    } else {
      setMembership((prev) => new Set([...prev, colId]));
    }
  };

  const addBulk = async (colId: string) => {
    if (!sceneIds?.length) return;
    setBusy(true);
    setError(null);
    try {
      let added = 0;
      for (const id of sceneIds) {
        await api.addSceneToCollection(id, colId);
        added += 1;
      }
      onBulkAdded?.(colId, added);
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to add to collection");
    } finally {
      setBusy(false);
    }
  };

  const title = bulk ? `Add ${sceneIds?.length ?? 0} scene(s) to collection` : "Add to collection";
  const subtitle = bulk ? undefined : scene?.title;

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="!max-w-sm">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>
        {subtitle && (
          <p className="text-sm text-[var(--color-muted-foreground)] mb-2">{subtitle}</p>
        )}
        {error && <p className="mb-2 text-sm text-red-500">{error}</p>}
        {loading ? (
          <p className="text-xs text-[var(--color-muted-foreground)]">Loading...</p>
        ) : (
          <div className="space-y-1 max-h-48 overflow-y-auto">
            {collections.filter((c) => c.type !== "smart").length === 0 ? (
              <p className="text-xs text-[var(--color-muted-foreground)]">
                No collections available. Create one first.
              </p>
            ) : (
              collections
                .filter((col) => col.type !== "smart")
                .map((col) => {
                  const isActive = !bulk && membership.has(col.id);
                  const Icon = col.type === "watchlist" ? ListTodo : FolderSymlink;
                  return (
                    <label
                      key={col.id}
                      className="flex items-center justify-between rounded-md border border-[var(--color-border)] p-2 cursor-pointer hover:bg-[var(--color-muted)]/30"
                    >
                      <div className="flex items-center gap-2">
                        <Icon className="h-4 w-4 text-[var(--color-muted-foreground)]" />
                        <span className="font-medium">{col.name}</span>
                      </div>
                      {bulk ? (
                        <button
                          type="button"
                          className="rounded-md border border-[var(--color-border)] px-2 py-1 text-xs hover:bg-[var(--color-muted)]"
                          disabled={busy}
                          onClick={() => void addBulk(col.id)}
                        >
                          Add
                        </button>
                      ) : (
                        <input
                          type="checkbox"
                          checked={isActive}
                          onChange={() => toggle(col.id)}
                          className="h-4 w-4 cursor-pointer"
                        />
                      )}
                    </label>
                  );
                })
            )}
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
