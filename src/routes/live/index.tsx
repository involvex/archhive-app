import { createFileRoute, Link } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { MediaItem } from "@/lib/types";
import { SceneCard } from "@/components/SceneCard";
import { BrowseItemDetailsDialog } from "@/components/BrowseItemDetailsDialog";
import { ErrorState } from "@/components/ErrorState";
import { SkeletonGrid } from "@/components/SkeletonGrid";
import { EmptyState } from "@/components/EmptyState";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Radio, Search } from "lucide-react";

export const Route = createFileRoute("/live/")({
  component: LiveIndexPage,
});

function LiveIndexPage() {
  const [items, setItems] = useState<MediaItem[]>([]);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [infoItem, setInfoItem] = useState<MediaItem | null>(null);

  const loadPopular = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const result = await api.browse("chaturbate", "livestream", "", 1);
      setItems(result.items);
    } catch (e) {
      const raw = e instanceof Error ? e.message : "Failed to load streams";
      console.error("[live] loadPopular failed:", e);
      setError(raw.replace(/^site error:\s*/i, ""));
    } finally {
      setLoading(false);
    }
  }, []);

  const loadSearch = useCallback(async () => {
    if (!query.trim()) return;
    setLoading(true);
    setError("");
    try {
      const result = await api.browse("chaturbate", "search", query.trim(), 1);
      setItems(result.items);
    } catch (e) {
      const raw = e instanceof Error ? e.message : "Search failed";
      console.error("[live] loadSearch failed:", e);
      setError(raw.replace(/^site error:\s*/i, ""));
    } finally {
      setLoading(false);
    }
  }, [query]);

  useEffect(() => {
    void loadPopular();
  }, [loadPopular]);

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <Radio className="h-5 w-5 text-red-500" />
        <h2 className="text-2xl font-bold">Live Streams</h2>
      </div>

      <div className="flex gap-2 max-w-md">
        <Input
          placeholder="Search Chaturbate..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") void loadSearch();
          }}
        />
        <Button onClick={loadSearch} disabled={loading}>
          Search
        </Button>
      </div>

      {error && (
        <>
          <ErrorState message={error} onRetry={loadPopular} />
          <p className="text-xs text-red-300/80">
            Chaturbate lists are scraped server-side without yt-dlp. If you repeatedly see no rooms,
            make sure cookies for <code className="font-mono">chaturbate</code> are configured in{" "}
            <Link to="/settings" className="underline">
              Settings &rarr; Cookies
            </Link>
            .
          </p>
        </>
      )}

      <div className="grid grid-cols-3 sm:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
        {items.map((item) => (
          <Link
            key={item.id}
            to="/live/$site/$slug"
            params={{ site: "chaturbate", slug: item.channel ?? item.performers[0] ?? "" }}
          >
            <SceneCard item={item} onInfo={(i) => setInfoItem(i)} />
          </Link>
        ))}
      </div>

      {loading && !error && <SkeletonGrid count={12} cols={3} />}

      {!loading && items.length === 0 && !error && (
        <EmptyState
          icon={<Search className="h-8 w-8" />}
          title="No streams found"
          description="Try a different search term."
        />
      )}

      <BrowseItemDetailsDialog
        item={infoItem}
        open={infoItem !== null}
        onClose={() => setInfoItem(null)}
      />
    </div>
  );
}
