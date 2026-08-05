import { createFileRoute } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useState } from "react";
import { api } from "@/lib/api/client";
import type { MediaItem } from "@/lib/types";
import { SceneCard } from "@/components/SceneCard";
import { BrowseItemDetailsDialog } from "@/components/BrowseItemDetailsDialog";
import { UrlPlayerDialog } from "@/components/UrlPlayerDialog";
import { ErrorState } from "@/components/ErrorState";
import { SkeletonGrid } from "@/components/SkeletonGrid";
import { EmptyState } from "@/components/EmptyState";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { LayoutGrid, Link2, List, Search } from "lucide-react";
import { browseCacheKey, useBrowseStore } from "@/lib/stores/browse";
import { useInfiniteScroll } from "@/lib/hooks/useInfiniteScroll";
import { usePullToRefresh } from "@/lib/hooks/usePullToRefresh";

export const Route = createFileRoute("/browse/by-url")({
  validateSearch: (search: Record<string, unknown>): { url?: string } => {
    const u = search.url;
    return { url: typeof u === "string" ? u : undefined };
  },
  component: CustomBrowsePage,
});

function CustomBrowsePage() {
  const searchParams = Route.useSearch();
  const cacheKey = useMemo(() => browseCacheKey({ site: "custom", kind: "by-url", url: "_" }), []);
  const cached = useBrowseStore((s) => s.caches[cacheKey]);
  const setCache = useBrowseStore((s) => s.set);

  const initialUrl = searchParams.url ?? cached?.url ?? "";
  const [url, setUrl] = useState(initialUrl);
  const [items, setItems] = useState<MediaItem[]>(cached?.items ?? []);
  const [page, setPage] = useState(cached?.page ?? 1);
  const [hasMore, setHasMore] = useState(cached?.hasMore ?? false);
  const [loading, setLoading] = useState(!!initialUrl);
  const [error, setError] = useState("");
  const [infoItem, setInfoItem] = useState<MediaItem | null>(null);
  const [watchItem, setWatchItem] = useState<MediaItem | null>(null);
  const [viewMode, setViewMode] = useState<"grid" | "list">("grid");

  useEffect(() => {
    setCache(cacheKey, {
      items,
      page,
      hasMore,
      querySlug: url,
      url,
    });
  }, [cacheKey, items, page, hasMore, url, setCache]);

  const load = useCallback(
    async (p: number, append = false) => {
      const trimmed = url.trim();
      if (!trimmed) return;
      setLoading(true);
      setError("");
      try {
        const result = await api.browse("custom", "video", trimmed, p);
        setItems((prev) => (append ? [...prev, ...result.items] : result.items));
        setHasMore(result.has_more);
        setPage(p);
      } catch (e) {
        const msg = e instanceof Error ? e.message : "Browse failed";
        setError(msg.replace(/^site error:\s*/i, ""));
        if (!append) setItems([]);
      } finally {
        setLoading(false);
      }
    },
    [url],
  );

  const loadMore = useCallback(() => {
    void load(page + 1, true);
  }, [page, load]);

  const { sentinelRef } = useInfiniteScroll(loadMore, hasMore, loading);

  const handleRefresh = useCallback(async () => {
    void load(page, false);
  }, [load, page]);

  const { containerRef, pullDistance, refreshing } = usePullToRefresh({
    onRefresh: handleRefresh,
    disabled: false,
  });

  async function handleDownload(item: MediaItem) {
    await api.queueDownload(item.url, "custom");
  }

  return (
    <div ref={containerRef} className="space-y-4">
      {pullDistance > 0 && (
        <div
          className="flex items-center justify-center transition-height overflow-hidden"
          style={{ height: pullDistance }}
        >
          <div
            className={`text-xs transition-opacity ${pullDistance > 60 ? "text-[var(--color-primary)]" : "text-[var(--color-muted-foreground)]"}`}
          >
            {refreshing
              ? "Refreshing..."
              : pullDistance > 60
                ? "Release to refresh"
                : "Pull to refresh"}
          </div>
        </div>
      )}

      <div>
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 className="flex items-center gap-2 text-2xl font-bold">
            <Link2 className="h-6 w-6" />
            Custom URL
          </h2>
          {items.length > 0 && (
            <div className="flex rounded-md border border-[var(--color-border)] overflow-hidden">
              <button
                type="button"
                onClick={() => setViewMode("grid")}
                className={`p-1.5 transition-colors ${viewMode === "grid" ? "bg-[var(--color-muted)] text-[var(--color-foreground)]" : "text-[var(--color-muted-foreground)] hover:text-[var(--color-foreground)]"}`}
                aria-label="Grid view"
              >
                <LayoutGrid className="h-4 w-4" />
              </button>
              <button
                type="button"
                onClick={() => setViewMode("list")}
                className={`p-1.5 transition-colors ${viewMode === "list" ? "bg-[var(--color-muted)] text-[var(--color-foreground)]" : "text-[var(--color-muted-foreground)] hover:text-[var(--color-foreground)]"}`}
                aria-label="List view"
              >
                <List className="h-4 w-4" />
              </button>
            </div>
          )}
        </div>
        <p className="text-sm text-[var(--color-muted-foreground)]">
          Paste a profile, playlist, or channel URL — yt-dlp lists entries on the desktop host.
        </p>
        <div className="mt-3 flex gap-2 max-w-2xl">
          <Input
            placeholder="https://www.tiktok.com/@user"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && void load(1)}
          />
          <Button onClick={() => void load(1)} disabled={loading}>
            Browse
          </Button>
        </div>
      </div>

      {error && <ErrorState message={error} onRetry={() => void load(page, false)} />}

      {loading && items.length === 0 && <SkeletonGrid count={12} />}

      {!loading && !error && items.length === 0 && url && (
        <EmptyState
          icon={<Search className="h-8 w-8" />}
          title="No results found"
          description="No media items were found for this URL."
        />
      )}

      {viewMode === "grid" && (
        <div className="grid grid-cols-3 sm:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
          {items.map((item) => (
            <SceneCard
              key={item.id}
              item={item}
              onDownload={(i) => void handleDownload(i)}
              onInfo={setInfoItem}
              onWatch={setWatchItem}
            />
          ))}
        </div>
      )}

      {viewMode === "list" && (
        <div className="flex flex-col gap-2">
          {items.map((item) => (
            <SceneCard
              key={item.id}
              item={item}
              listView
              onDownload={(i) => void handleDownload(i)}
              onInfo={setInfoItem}
              onWatch={setWatchItem}
            />
          ))}
        </div>
      )}

      {hasMore && <div ref={sentinelRef} className="h-4" />}

      <BrowseItemDetailsDialog
        item={infoItem}
        open={infoItem !== null}
        onClose={() => setInfoItem(null)}
      />

      <UrlPlayerDialog
        item={watchItem}
        open={watchItem !== null}
        onClose={() => setWatchItem(null)}
      />
    </div>
  );
}
