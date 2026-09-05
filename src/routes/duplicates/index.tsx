import { createFileRoute } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { DuplicateGroup } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { DuplicateMergeCard } from "@/components/DuplicateMergeCard";
import { SkeletonGrid } from "@/components/SkeletonGrid";
import { EmptyState } from "@/components/EmptyState";
import { RefreshCw, Search, Trash2 } from "lucide-react";

export const Route = createFileRoute("/duplicates/")({
  component: DuplicatesPage,
});

function DuplicatesPage() {
  const [groups, setGroups] = useState<DuplicateGroup[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");

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
        <div className="flex gap-2">
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
