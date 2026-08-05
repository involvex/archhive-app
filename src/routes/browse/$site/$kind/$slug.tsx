import { createFileRoute, Link } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useState } from "react";
import { LayoutGrid, List, Search } from "lucide-react";
import { api } from "@/lib/api/client";
import { normalizeBrowseInput } from "@/lib/browse/normalize";
import type { BrowseKind, MediaItem } from "@/lib/types";
import { SceneCard } from "@/components/SceneCard";
import { BrowseItemDetailsDialog } from "@/components/BrowseItemDetailsDialog";
import { UrlPlayerDialog } from "@/components/UrlPlayerDialog";
import { SkeletonGrid } from "@/components/SkeletonGrid";
import { ErrorState } from "@/components/ErrorState";
import { EmptyState } from "@/components/EmptyState";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { useInfiniteScroll } from "@/lib/hooks/useInfiniteScroll";
import { usePullToRefresh } from "@/lib/hooks/usePullToRefresh";
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
  const [initial, setInitial] = useState(!cached?.items?.length);
  const [error, setError] = useState("");
  const [infoItem, setInfoItem] = useState<MediaItem | null>(null);
  const [watchItem, setWatchItem] = useState<MediaItem | null>(null);
  const [viewMode, setViewMode] = useState<"grid" | "list">("grid");

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
        setInitial(false);
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

  const loadMore = useCallback(() => {
    if (!loading && hasMore) {
      void load(page + 1, true);
    }
  }, [loading, hasMore, page, load]);

  const { sentinelRef } = useInfiniteScroll(loadMore, hasMore, loading);

  const handleRefresh = useCallback(async () => {
    void load(1, false);
  }, [load]);

  const { containerRef, pullDistance, refreshing } = usePullToRefresh({
    onRefresh: handleRefresh,
    disabled: false,
  });

  async function handleDownload(item: MediaItem) {
    await api.queueDownload(item.url, site);
  }

  const showInitialSkeleton = initial && loading && !error;
  const showEmpty = !loading && !error && items.length === 0 && querySlug && !initial;
  const showItems = items.length > 0;

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
        <Link
          to="/browse"
          className="text-xs text-[var(--color-muted-foreground)] hover:text-[var(--color-foreground)] transition-colors"
        >
          &larr; Browse
        </Link>
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 className="text-2xl font-bold capitalize">
            {site} / {kind}
          </h2>
          {showItems && (
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
        <div className="mt-3 flex gap-2 max-w-md">
          <Input
            placeholder={`Enter ${kind} slug...`}
            value={querySlug}
            onChange={(e) => setQuerySlug(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                setInitial(true);
                setItems([]);
                void load(1);
              }
            }}
          />
          <Button
            onClick={() => {
              setInitial(true);
              setItems([]);
              void load(1);
            }}
            disabled={loading}
          >
            Browse
          </Button>
        </div>
      </div>

      {error && <ErrorState message={error} onRetry={() => void load(page, false)} />}

      {showInitialSkeleton && <SkeletonGrid count={12} cols={3} />}

      {showItems && viewMode === "grid" && (
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

      {showItems && viewMode === "list" && (
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

      {showItems && hasMore && <div ref={sentinelRef} className="h-4" />}

      {loading && showItems && (
        <div className="flex justify-center py-4 text-xs text-[var(--color-muted-foreground)]">
          Loading more...
        </div>
      )}

      {showEmpty && (
        <EmptyState
          icon={<Search className="h-8 w-8" />}
          title="No items found"
          description="Try a different search term or browse another category."
        />
      )}

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
