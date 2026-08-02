import { createFileRoute, Link } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { MediaItem } from "@/lib/types";
import { SceneCard } from "@/components/SceneCard";
import { BrowseItemDetailsDialog } from "@/components/BrowseItemDetailsDialog";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Radio, Settings, AlertTriangle } from "lucide-react";

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
        <div className="flex items-start gap-2 rounded-md border border-red-400/30 bg-red-400/10 px-3 py-2 text-sm text-red-400">
          <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
          <div className="space-y-1">
            <p>{error}</p>
            <p className="text-xs text-red-300/80">
              Chaturbate lists are scraped server-side without yt-dlp. If you repeatedly see no
              rooms, make sure cookies for <code className="font-mono">chaturbate</code> are
              configured in{" "}
              <Link to="/settings" className="underline">
                Settings &rarr; Cookies
              </Link>
              .
            </p>
          </div>
        </div>
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

      {!loading && items.length === 0 && !error && (
        <div className="flex flex-col items-start gap-2 rounded-md border border-[var(--color-border)] bg-[var(--color-muted)]/30 px-4 py-3 text-sm text-[var(--color-muted-foreground)]">
          <p>No live streams found.</p>
          <p className="flex items-center gap-1.5 text-xs">
            <Settings className="h-3.5 w-3.5" />
            <Link to="/settings" className="underline">
              Configure Chaturbate cookies
            </Link>{" "}
            for the public room list to load reliably.
          </p>
        </div>
      )}

      <BrowseItemDetailsDialog
        item={infoItem}
        open={infoItem !== null}
        onClose={() => setInfoItem(null)}
      />
    </div>
  );
}
