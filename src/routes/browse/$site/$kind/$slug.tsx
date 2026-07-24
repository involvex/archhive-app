import { createFileRoute } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useState } from "react";
import { api } from "@/lib/api/client";
import { normalizeBrowseInput } from "@/lib/browse/normalize";
import type { BrowseKind, MediaItem } from "@/lib/types";
import { SceneCard } from "@/components/SceneCard";
import { BrowseItemDetailsDialog } from "@/components/BrowseItemDetailsDialog";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { browseCacheKey, useBrowseStore } from "@/lib/stores/browse";

export const Route = createFileRoute("/browse/$site/$kind/$slug")({
  component: BrowseDetailPage,
});

function BrowseDetailPage() {
  const { site, kind, slug } = Route.useParams();
  const cacheKey = useMemo(
    () => browseCacheKey({ site, kind, slug: slug === "example" ? "" : slug }),
    [site, kind, slug],
  );
  const cached = useBrowseStore((s) => s.caches[cacheKey]);
  const setCache = useBrowseStore((s) => s.set);

  const [items, setItems] = useState<MediaItem[]>(cached?.items ?? []);
  const [page, setPage] = useState(cached?.page ?? 1);
  const [hasMore, setHasMore] = useState(cached?.hasMore ?? false);
  const [querySlug, setQuerySlug] = useState(cached?.querySlug ?? (slug === "example" ? "" : slug));
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [infoItem, setInfoItem] = useState<MediaItem | null>(null);

  useEffect(() => {
    setCache(cacheKey, { items, page, hasMore, querySlug });
  }, [cacheKey, items, page, hasMore, querySlug, setCache]);

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
          <SceneCard
            key={item.id}
            item={item}
            onDownload={(i) => void handleDownload(i)}
            onInfo={setInfoItem}
          />
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

      <BrowseItemDetailsDialog
        item={infoItem}
        open={infoItem !== null}
        onClose={() => setInfoItem(null)}
      />
    </div>
  );
}
