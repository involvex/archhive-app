import { useState } from "react";
import { api } from "@/lib/api/client";
import type { Scene } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { X } from "lucide-react";

interface SceneEditDialogProps {
  scene: Scene | null;
  open: boolean;
  onClose: () => void;
  onSaved: (scene: Scene) => void;
  onDeleted?: (id: string) => void;
}

function SceneEditForm({
  scene,
  onClose,
  onSaved,
  onDeleted,
}: {
  scene: Scene;
  onClose: () => void;
  onSaved: (scene: Scene) => void;
  onDeleted?: (id: string) => void;
}) {
  const [title, setTitle] = useState(scene.title);
  const [performers, setPerformers] = useState(scene.performers.join(", "));
  const [tags, setTags] = useState(scene.tags.join(", "));
  const [renameFile, setRenameFile] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);

  async function handleSave() {
    setSaving(true);
    setError(null);
    try {
      const performerList = performers
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean);
      const tagList = tags
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean);
      const updated = await api.updateScene(scene.id, {
        title: title.trim() || scene.title,
        performers: performerList,
        tags: tagList,
        rename_file: renameFile,
      });
      onSaved(updated);
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Save failed");
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete(deleteFiles: boolean) {
    setDeleting(true);
    setError(null);
    try {
      await api.deleteScene(scene.id, deleteFiles);
      onDeleted?.(scene.id);
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Delete failed");
    } finally {
      setDeleting(false);
    }
  }

  return (
    <>
      <div className="mb-4 flex items-center justify-between">
        <h3 className="text-lg font-semibold">Edit scene</h3>
        <button
          type="button"
          onClick={onClose}
          className="rounded p-1 hover:bg-[var(--color-muted)]"
        >
          <X className="h-4 w-4" />
        </button>
      </div>
      <div className="space-y-3">
        <div>
          <label className="mb-1 block text-xs text-[var(--color-muted-foreground)]">Title</label>
          <Input value={title} onChange={(e) => setTitle(e.target.value)} />
        </div>
        <div>
          <label className="mb-1 block text-xs text-[var(--color-muted-foreground)]">
            Performers (comma-separated)
          </label>
          <Input value={performers} onChange={(e) => setPerformers(e.target.value)} />
        </div>
        <div>
          <label className="mb-1 block text-xs text-[var(--color-muted-foreground)]">
            Tags (comma-separated)
          </label>
          <Input value={tags} onChange={(e) => setTags(e.target.value)} />
        </div>
        {scene.path && (
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={renameFile}
              onChange={(e) => setRenameFile(e.target.checked)}
            />
            Rename file on disk to match title
          </label>
        )}
        {error && <p className="text-sm text-red-500">{error}</p>}
      </div>

      {confirmDelete ? (
        <div className="mt-4 space-y-2 rounded-md border border-red-400/30 bg-red-400/10 p-3">
          <p className="text-sm">Delete this scene?</p>
          <div className="flex flex-wrap gap-2">
            <Button
              size="sm"
              variant="outline"
              onClick={() => setConfirmDelete(false)}
              disabled={deleting}
            >
              Cancel
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={() => void handleDelete(false)}
              disabled={deleting}
            >
              Remove from library
            </Button>
            <Button
              size="sm"
              variant="destructive"
              onClick={() => void handleDelete(true)}
              disabled={deleting}
            >
              Also delete file
            </Button>
          </div>
        </div>
      ) : (
        <div className="mt-4 flex justify-between gap-2">
          <Button
            variant="outline"
            className="text-red-400"
            onClick={() => setConfirmDelete(true)}
            disabled={saving}
          >
            Delete…
          </Button>
          <div className="flex gap-2">
            <Button variant="outline" onClick={onClose} disabled={saving}>
              Cancel
            </Button>
            <Button onClick={() => void handleSave()} disabled={saving}>
              {saving ? "Saving…" : "Save"}
            </Button>
          </div>
        </div>
      )}
    </>
  );
}

export function SceneEditDialog({
  scene,
  open,
  onClose,
  onSaved,
  onDeleted,
}: SceneEditDialogProps) {
  if (!open || !scene) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
      <div
        role="dialog"
        aria-modal="true"
        className="w-full max-w-md rounded-lg border border-[var(--color-border)] bg-[var(--color-card)] p-4 shadow-xl"
      >
        <SceneEditForm
          key={scene.id}
          scene={scene}
          onClose={onClose}
          onSaved={onSaved}
          onDeleted={onDeleted}
        />
      </div>
    </div>
  );
}
