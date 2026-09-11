import { useState, useEffect } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { api } from "@/lib/api/client";
import type { Collection, Scene } from "@/lib/types";
import { FolderSymlink, ListTodo } from "lucide-react";

interface CollectionPickerDialogProps {
  scene: Scene | null;
  open: boolean;
  onClose: () => void;
  collections: Collection[];
  onCollectionChange: (collectionId: string | null, scene: Scene) => void;
}

export function CollectionPickerDialog({
  scene,
  open,
  onClose,
  collections,
  onCollectionChange,
}: CollectionPickerDialogProps) {
  const [membership, setMembership] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!scene) return;
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setLoading(true);
    api
      .sceneCollectionIds(scene.id)
      .then((ids) => setMembership(new Set(ids)))
      .catch(() => setMembership(new Set()))
      .finally(() => setLoading(false));
  }, [scene]);

  if (!scene) return null;

  const toggle = (colId: string) => {
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

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="!max-w-sm">
        <DialogHeader>
          <DialogTitle>Add to collection</DialogTitle>
        </DialogHeader>
        <p className="text-sm text-[var(--color-muted-foreground)] mb-2">{scene.title}</p>
        {loading ? (
          <p className="text-xs text-[var(--color-muted-foreground)]">Loading...</p>
        ) : (
          <div className="space-y-1 max-h-48 overflow-y-auto">
            {collections.length === 0 ? (
              <p className="text-xs text-[var(--color-muted-foreground)]">
                No collections available. Create one first.
              </p>
            ) : (
              collections.map((col) => {
                const isActive = membership.has(col.id);
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
                    <input
                      type="checkbox"
                      checked={isActive}
                      onChange={() => toggle(col.id)}
                      className="h-4 w-4 cursor-pointer"
                    />
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
