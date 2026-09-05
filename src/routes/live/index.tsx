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
import { cn } from "@/lib/utils";

export const Route = createFileRoute("/live/")({
  component: LiveIndexPage,
});

const LIVE_SITES = [
  { id: "chaturbate", label: "Chaturbate" },
  { id: "stripchat", label: "Stripchat" },
] as const;

function LiveIndexPage() {
  const [items, setItems] = useState<MediaItem[]>([]);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [infoItem, setInfoItem] = useState<MediaItem | null>(null);
  const [selectedSite, setSelectedSite] = useState<string>("chaturbate");

  const loadPopular = useCallback(async (site: string) => {
    setLoading(true);
    setError("");
    try {
      const result = await api.browse(site, "livestream", "", 1);
      setItems(result.items);
    } catch (e) {
      const raw = e instanceof Error ? e.message : "Failed to load streams";
      console.error("[live] loadPopular failed:", e);
      setError(raw.replace(/^site error:\s*/i, ""));
    } finally {
      setLoading(false);
    }
  }, []);

  const loadSearch = useCallback(
    async (site: string) => {
      if (!query.trim()) return;
      setLoading(true);
      setError("");
      try {
        const result = await api.browse(site, "search", query.trim(), 1);
        setItems(result.items);
      } catch (e) {
        const raw = e instanceof Error ? e.message : "Search failed";
        console.error("[live] loadSearch failed:", e);
        setError(raw.replace(/^site error:\s*/i, ""));
      } finally {
        setLoading(false);
      }
    },
    [query],
  );

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void loadPopular(selectedSite);
  }, [loadPopular, selectedSite]);

  const handleSiteChange = useCallback((site: string) => {
    setSelectedSite(site);
    setQuery("");
    setError("");
  }, []);

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <Radio className="h-5 w-5 text-red-500" />
        <h2 className="text-2xl font-bold">Live Streams</h2>
      </div>

      {/* Site selector chips */}
      <div className="flex gap-2">
        {LIVE_SITES.map((site) => (
          <button
            key={site.id}
            type="button"
            onClick={() => handleSiteChange(site.id)}
            className={cn(
              "rounded-full px-4 py-1.5 text-sm font-medium transition-colors",
              selectedSite === site.id
                ? "bg-[var(--color-primary)] text-[var(--color-primary-foreground)]"
                : "bg-[var(--color-muted)] text-[var(--color-muted-foreground)] hover:bg-[var(--color-accent)]",
            )}
          >
            {site.label}
          </button>
        ))}
      </div>

      {/* Search */}
      <div className="flex gap-2 max-w-md">
        <Input
          placeholder={`Search ${LIVE_SITES.find((s) => s.id === selectedSite)?.label ?? "streams"}...`}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") void loadSearch(selectedSite);
          }}
        />
        <Button onClick={() => loadSearch(selectedSite)} disabled={loading}>
          Search
        </Button>
      </div>

      {error && (
        <>
          <ErrorState message={error} onRetry={() => loadPopular(selectedSite)} />
          <p className="text-xs text-red-300/80">
            Live site lists are scraped server-side. If you repeatedly see no rooms, make sure
            cookies for <code className="font-mono">{selectedSite}</code> are configured in{" "}
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
            params={{
              site: selectedSite,
              slug: item.channel ?? item.performers[0] ?? "",
            }}
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
          description="Try a different search term or site."
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
