import { createFileRoute } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useState } from "react";
import { api } from "@/lib/api/client";
import type { MediaItem } from "@/lib/types";
import { SceneCard } from "@/components/SceneCard";
import { BrowseItemDetailsDialog } from "@/components/BrowseItemDetailsDialog";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Link2 } from "lucide-react";
import { browseCacheKey, useBrowseStore } from "@/lib/stores/browse";

export const Route = createFileRoute("/browse/by-url")({
  component: CustomBrowsePage,
});

function CustomBrowsePage() {
  const cacheKey = useMemo(() => browseCacheKey({ site: "custom", kind: "by-url", url: "_" }), []);
  const cached = useBrowseStore((s) => s.caches[cacheKey]);
  const setCache = useBrowseStore((s) => s.set);

  const [url, setUrl] = useState(cached?.url ?? "");
  const [items, setItems] = useState<MediaItem[]>(cached?.items ?? []);
  const [page, setPage] = useState(cached?.page ?? 1);
  const [hasMore, setHasMore] = useState(cached?.hasMore ?? false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [infoItem, setInfoItem] = useState<MediaItem | null>(null);

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

  async function handleDownload(item: MediaItem) {
    await api.queueDownload(item.url, "custom");
  }

  return (
    <div className="space-y-4">
      <div>
        <h2 className="flex items-center gap-2 text-2xl font-bold">
          <Link2 className="h-6 w-6" />
          Custom URL
        </h2>
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

      {error && (
        <p className="rounded-md border border-red-400/30 bg-red-400/10 px-3 py-2 text-sm text-red-400">
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

      <BrowseItemDetailsDialog
        item={infoItem}
        open={infoItem !== null}
        onClose={() => setInfoItem(null)}
      />
    </div>
  );
}
