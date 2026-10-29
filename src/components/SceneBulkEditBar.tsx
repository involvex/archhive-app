import { useState } from "react";
import { api } from "@/lib/api/client";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

interface SceneBulkEditBarProps {
  selectedIds: string[];
  onClear: () => void;
  onApplied: () => void;
  /** Media vault is enabled in settings, so "Move to vault" is offered. */
  canVault?: boolean;
  /** Moves every selected scene into the vault; resolves false on failure. */
  onMoveToVault?: () => Promise<boolean>;
}

export function SceneBulkEditBar({
  selectedIds,
  onClear,
  onApplied,
  canVault = false,
  onMoveToVault,
}: SceneBulkEditBarProps) {
  const [performers, setPerformers] = useState("");
  const [tags, setTags] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (selectedIds.length === 0) return null;

  // #26: bulk watched toggle.
  async function handleMarkWatched(watched: boolean) {
    setSaving(true);
    setError(null);
    try {
      await api.markWatched(selectedIds, watched);
      onApplied();
      onClear();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Bulk update failed");
    } finally {
      setSaving(false);
    }
  }

  async function handleApply() {
    const performersAdd = performers
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
    const tagsAdd = tags
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
    if (performersAdd.length === 0 && tagsAdd.length === 0) {
      setError("Enter performers or tags to add.");
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await api.batchUpdateScenes({
        scene_ids: selectedIds,
        performers_add: performersAdd.length ? performersAdd : undefined,
        tags_add: tagsAdd.length ? tagsAdd : undefined,
      });
      setPerformers("");
      setTags("");
      onApplied();
      onClear();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Bulk update failed");
    } finally {
      setSaving(false);
    }
  }

  async function handleVault() {
    if (!onMoveToVault) return;
    setSaving(true);
    setError(null);
    try {
      // The caller owns the biometric prompt, the per-scene failures and the
      // toast; it returns false when nothing was moved so we keep the bar open.
      const moved = await onMoveToVault();
      if (moved) {
        onApplied();
        onClear();
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Move to vault failed");
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="sticky top-0 z-10 rounded-lg border border-[var(--color-border)] bg-[var(--color-card)] p-3 shadow-md">
      <div className="mb-2 flex items-center justify-between gap-2">
        <p className="text-sm font-medium">{selectedIds.length} scene(s) selected</p>
        <Button variant="ghost" size="sm" onClick={onClear}>
          Clear
        </Button>
      </div>
      <div className="grid gap-2 md:grid-cols-2">
        <Input
          placeholder="Add performers (comma-separated)"
          value={performers}
          onChange={(e) => setPerformers(e.target.value)}
        />
        <Input
          placeholder="Add tags (comma-separated)"
          value={tags}
          onChange={(e) => setTags(e.target.value)}
        />
      </div>
      {error && <p className="mt-2 text-sm text-red-500">{error}</p>}
      <div className="mt-2 flex flex-wrap justify-end gap-2">
        <Button variant="outline" onClick={() => void handleMarkWatched(true)} disabled={saving}>
          Mark watched
        </Button>
        <Button variant="outline" onClick={() => void handleMarkWatched(false)} disabled={saving}>
          Clear watched
        </Button>
        {canVault && onMoveToVault && (
          <Button variant="outline" onClick={() => void handleVault()} disabled={saving}>
            Move to vault
          </Button>
        )}
        <Button onClick={() => void handleApply()} disabled={saving}>
          {saving ? "Applying…" : "Apply to selected"}
        </Button>
      </div>
    </div>
  );
}
