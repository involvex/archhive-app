import { createFileRoute } from "@tanstack/react-router";
import { useCallback, useState } from "react";
import { api } from "@/lib/api/client";
import { normalizeBrowseInput } from "@/lib/browse/normalize";
import type { BrowseKind, MediaItem } from "@/lib/types";
import { SceneCard } from "@/components/SceneCard";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";

export const Route = createFileRoute("/browse/$site/$kind/$slug")({
  component: BrowseDetailPage,
});

function BrowseDetailPage() {
  const { site, kind, slug } = Route.useParams();
  const [items, setItems] = useState<MediaItem[]>([]);
  const [page, setPage] = useState(1);
  const [hasMore, setHasMore] = useState(false);
  const [querySlug, setQuerySlug] = useState(slug === "example" ? "" : slug);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const load = useCallback(
    async (p: number, append = false) => {
      if (!querySlug.trim()) return;
      setLoading(true);
      setError("");
      try {
        const normalized = normalizeBrowseInput(site, kind as BrowseKind, querySlug.trim());
        const result = await api.browse(site, normalized.kind, normalized.slug, p);
        setItems((prev) => (append ? [...prev, ...result.items] : result.items));
        setHasMore(result.has_more);
        setPage(p);
      } catch (e) {
        const msg = e instanceof Error ? e.message : "Browse failed";
        setError(msg.replace(/^site error:\s*/i, ""));
        if (!append) setItems([]);
        console.error(e);
      } finally {
        setLoading(false);
      }
    },
    [site, kind, querySlug],
  );

  async function handleDownload(item: MediaItem) {
    await api.queueDownload(item.url, site);
  }

  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-2xl font-bold capitalize">
          {site} / {kind}
        </h2>
        <div className="mt-3 flex gap-2 max-w-md">
          <Input
            placeholder={`Enter ${kind} slug...`}
            value={querySlug}
            onChange={(e) => setQuerySlug(e.target.value)}
          />
          <Button onClick={() => void load(1)} disabled={loading}>
            Browse
          </Button>
        </div>
      </div>

      {error && (
        <p className="text-sm text-red-400 rounded-md border border-red-400/30 bg-red-400/10 px-3 py-2">
          {error}
        </p>
      )}

      <div className="grid grid-cols-2 gap-3 md:grid-cols-3 lg:grid-cols-4">
        {items.map((item) => (
          <SceneCard key={item.id} item={item} onDownload={(i) => void handleDownload(i)} />
        ))}
      </div>

      {hasMore && (
        <Button variant="outline" onClick={() => void load(page + 1, true)} disabled={loading}>
          Load more
        </Button>
      )}

      {!loading && items.length === 0 && querySlug && (
        <p className="text-sm text-[var(--color-muted-foreground)]">No items found.</p>
      )}
    </div>
  );
}
