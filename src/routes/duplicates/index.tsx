import { createFileRoute } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import { pickBestDuplicateScene } from "@/lib/duplicates";
import type { DuplicateGroup } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { DuplicateMergeCard } from "@/components/DuplicateMergeCard";
import { SkeletonGrid } from "@/components/SkeletonGrid";
import { EmptyState } from "@/components/EmptyState";
import { RefreshCw, Search, Sparkles, Trash2 } from "lucide-react";

export const Route = createFileRoute("/duplicates/")({
  component: DuplicatesPage,
});

function DuplicatesPage() {
  const [groups, setGroups] = useState<DuplicateGroup[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [applyAllBusy, setApplyAllBusy] = useState(false);
  const [deleteFilesAll, setDeleteFilesAll] = useState(false);

  const loadGroups = useCallback(() => {
    setLoading(true);
    setError(null);
    void api
      .findDuplicates()
      .then(setGroups)
      .catch((e) => {
        console.error(e);
        setError(e instanceof Error ? e.message : "Failed to load duplicates");
      })
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void loadGroups();
  }, [loadGroups]);

  async function handleScan() {
    setScanning(true);
    setError(null);
    try {
      await api.scanLibrary();
      await loadGroups();
    } catch (e) {
      console.error(e);
      setError(e instanceof Error ? e.message : "Scan failed");
    } finally {
      setScanning(false);
    }
  }

  async function handleMerge(keepId: string, removeIds: string[], deleteFiles: boolean) {
    try {
      await api.mergeDuplicates(keepId, removeIds, deleteFiles);
      setGroups((prev) => prev.filter((g) => !g.scenes.some((s) => removeIds.includes(s.id))));
    } catch (e) {
      console.error(e);
      setError(e instanceof Error ? e.message : "Merge failed");
    }
  }

  async function handleDelete(sceneId: string) {
    try {
      await api.deleteScene(sceneId, false);
      setGroups((prev) =>
        prev
          .map((g) => ({
            ...g,
            scenes: g.scenes.filter((s) => s.id !== sceneId),
          }))
          .filter((g) => g.scenes.length > 1),
      );
    } catch (e) {
      console.error(e);
      setError(e instanceof Error ? e.message : "Delete failed");
    }
  }

  async function handleApplyKeepBestAll() {
    const targets = filteredGroups;
    if (targets.length === 0) return;
    if (
      !window.confirm(
        `Apply Keep best to ${targets.length} group(s)?${
          deleteFilesAll ? " Duplicate files will be deleted from disk." : ""
        }`,
      )
    ) {
      return;
    }
    setApplyAllBusy(true);
    setError(null);
    let merged = 0;
    try {
      for (const group of targets) {
        const best = pickBestDuplicateScene(group.scenes);
        if (!best) continue;
        const removeIds = group.scenes.filter((s) => s.id !== best.id).map((s) => s.id);
        if (removeIds.length === 0) continue;
        await api.mergeDuplicates(best.id, removeIds, deleteFilesAll);
        merged += 1;
        setGroups((prev) => prev.filter((g) => !g.scenes.some((s) => removeIds.includes(s.id))));
      }
    } catch (e) {
      console.error(e);
      setError(e instanceof Error ? e.message : "Apply keep-best failed");
    } finally {
      setApplyAllBusy(false);
      if (merged > 0 && !error) {
        // status via cleared groups is enough
      }
    }
  }

  const filteredGroups = groups.filter((g) =>
    g.scenes.some((s) => {
      const q = searchQuery.toLowerCase();
      return (
        s.title.toLowerCase().includes(q) ||
        s.performers.some((p) => p.toLowerCase().includes(q)) ||
        s.tags.some((t) => t.toLowerCase().includes(q))
      );
    }),
  );

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-2xl font-bold">Library — Duplicates</h2>
        <div className="flex flex-wrap gap-2">
          {filteredGroups.length > 0 && (
            <>
              <label className="flex items-center gap-1.5 text-xs text-[var(--color-muted-foreground)]">
                <input
                  type="checkbox"
                  checked={deleteFilesAll}
                  onChange={(e) => setDeleteFilesAll(e.target.checked)}
                />
                Delete files
              </label>
              <Button
                size="sm"
                variant="outline"
                onClick={() => void handleApplyKeepBestAll()}
                disabled={applyAllBusy || loading}
              >
                <Sparkles className="h-4 w-4 mr-1.5" />
                {applyAllBusy ? "Applying…" : `Keep best all (${filteredGroups.length})`}
              </Button>
            </>
          )}
          <Button size="sm" variant="outline" onClick={loadGroups} disabled={loading}>
            <RefreshCw className="h-4 w-4 mr-1.5" />
            Refresh
          </Button>
          <Button size="sm" onClick={handleScan} disabled={scanning}>
            <Search className="h-4 w-4 mr-1.5" />
            {scanning ? "Scanning…" : "Scan library"}
          </Button>
        </div>
      </div>

      {error && <p className="text-sm text-red-400">{error}</p>}

      {groups.length > 0 && (
        <Input
          placeholder="Search duplicates…"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="max-w-md"
        />
      )}

      {loading ? (
        <SkeletonGrid count={6} cols={2} />
      ) : filteredGroups.length === 0 ? (
        <EmptyState
          icon={<Trash2 className="h-8 w-8" />}
          title={groups.length === 0 ? "No duplicates found" : "No matches"}
          description={
            groups.length === 0
              ? "Run a library scan to detect duplicates by perceptual hash (pHash) or file hash (oShash)."
              : "No duplicates match your search."
          }
          action={
            groups.length === 0 ? (
              <Button size="sm" onClick={handleScan} disabled={scanning}>
                <Search className="h-4 w-4 mr-1.5" />
                {scanning ? "Scanning…" : "Scan library"}
              </Button>
            ) : undefined
          }
        />
      ) : (
        <div className="space-y-4">
          {filteredGroups.map((group) => (
            <DuplicateMergeCard
              key={group.hash + group.match_type}
              group={group}
              onMerge={handleMerge}
              onDelete={handleDelete}
            />
          ))}
        </div>
      )}
    </div>
  );
}
