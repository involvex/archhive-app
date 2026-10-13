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
import { Radio, Search, X } from "lucide-react";
import { cn } from "@/lib/utils";
import { SiteAvatar } from "@/components/SitePicker";
import { useMediaQuery } from "@/lib/hooks/useMediaQuery";

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
  const isPhone = useMediaQuery("(max-width: 767px)");
  const siteLabel = LIVE_SITES.find((s) => s.id === selectedSite)?.label ?? "streams";

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
        console.log("[live] Searching for:", query.trim());
        const result = await api.browse(site, "search", query.trim(), 1);
        console.log("[live] Search results:", result.items.length, "items");
        if (result.items.length > 0) {
          console.log("[live] First result:", result.items[0].title, result.items[0].channel);
        }
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

      {/* Site selector */}
      <div className="grid grid-cols-2 gap-2 sm:flex">
        {LIVE_SITES.map((site) => (
          <button
            key={site.id}
            type="button"
            onClick={() => handleSiteChange(site.id)}
            aria-pressed={selectedSite === site.id}
            className={cn(
              "flex min-h-12 items-center gap-2.5 rounded-xl px-3 py-2 text-sm font-medium transition active:scale-[0.97] sm:min-w-44",
              selectedSite === site.id
                ? "bg-[var(--color-primary)]/15 ring-2 ring-[var(--color-primary)]"
                : "bg-[var(--color-secondary)] hover:bg-[var(--color-muted)]",
            )}
          >
            <SiteAvatar site={{ id: site.id, display_name: site.label }} size="sm" />
            {site.label}
          </button>
        ))}
      </div>

      {/* Search */}
      <form
        className="flex gap-2 md:max-w-md"
        onSubmit={(e) => {
          e.preventDefault();
          (document.activeElement as HTMLElement | null)?.blur();
          void loadSearch(selectedSite);
        }}
      >
        <div className="relative min-w-0 flex-1">
          <Input
            className="h-11 pr-9 text-base md:h-9 md:text-sm"
            placeholder={`Search ${siteLabel} rooms…`}
            value={query}
            enterKeyHint="search"
            onChange={(e) => setQuery(e.target.value)}
          />
          {query && (
            <button
              type="button"
              aria-label="Clear search"
              className="absolute right-1 top-1/2 -translate-y-1/2 rounded p-2 text-[var(--color-muted-foreground)]"
              onClick={() => {
                setQuery("");
                void loadPopular(selectedSite);
              }}
            >
              <X className="h-4 w-4" />
            </button>
          )}
        </div>
        <Button
          type="submit"
          disabled={loading || !query.trim()}
          className="h-11 w-11 shrink-0 p-0 md:h-9 md:w-auto md:px-4"
          aria-label="Search"
        >
          <Search className="h-4 w-4 md:mr-1.5" />
          <span className="hidden md:inline">Search</span>
        </Button>
      </form>

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

      <div className="grid grid-cols-2 gap-2.5 sm:grid-cols-3 md:grid-cols-4 md:gap-3 lg:grid-cols-5 xl:grid-cols-6">
        {items.map((item) => (
          <Link
            key={item.id}
            to="/live/$site/$slug"
            params={{
              site: selectedSite,
              slug: item.channel ?? item.performers[0] ?? "",
            }}
            search={{
              fromSearch: query.trim() ? "1" : "0",
              searchQuery: query.trim(),
            }}
          >
            <SceneCard item={item} onInfo={(i) => setInfoItem(i)} compactActions={isPhone} />
          </Link>
        ))}
      </div>

      {loading && !error && <SkeletonGrid count={12} cols={isPhone ? 2 : 4} />}

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
