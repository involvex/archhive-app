import { createFileRoute, Link } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { MediaItem } from "@/lib/types";
import { SceneCard } from "@/components/SceneCard";
import { BrowseItemDetailsDialog } from "@/components/BrowseItemDetailsDialog";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Radio } from "lucide-react";

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
      const msg = e instanceof Error ? e.message : "Failed to load streams";
      setError(msg.replace(/^site error:\s*/i, ""));
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
      const msg = e instanceof Error ? e.message : "Search failed";
      setError(msg.replace(/^site error:\s*/i, ""));
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
        <p className="text-sm text-red-400 rounded-md border border-red-400/30 bg-red-400/10 px-3 py-2">
          {error}
        </p>
      )}

      <div className="grid grid-cols-2 gap-3 md:grid-cols-3 lg:grid-cols-4">
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

      {loading && <p className="text-sm text-[var(--color-muted-foreground)]">Loading...</p>}

      {!loading && items.length === 0 && (
        <p className="text-sm text-[var(--color-muted-foreground)]">No live streams found.</p>
      )}

      <BrowseItemDetailsDialog
        item={infoItem}
        open={infoItem !== null}
        onClose={() => setInfoItem(null)}
      />
    </div>
  );
}
