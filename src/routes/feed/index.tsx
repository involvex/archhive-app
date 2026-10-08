import { Link, createFileRoute, useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import { getPluginBrowseSites } from "@/lib/plugins/loader";
import { mergeSiteLists } from "@/lib/sites/catalog";
import { useSettingsStore } from "@/lib/stores/settings";
import type { MediaItem, Scene, SiteInfo } from "@/lib/types";
import { toWatchMap, watchFor, type WatchMap } from "@/lib/watch";
import { sceneThumbUrl } from "@/lib/mediaUrl";
import { SceneCard } from "@/components/SceneCard";
import { SkeletonGrid } from "@/components/SkeletonGrid";
import { ErrorState } from "@/components/ErrorState";
import { EmptyState } from "@/components/EmptyState";
import { DownloadProgressRow } from "@/components/DownloadProgress";
import { ScenePlayerDialog } from "@/components/ScenePlayerDialog";
import { useRecentlyViewedStore } from "@/lib/stores/recentlyViewed";
import { usePullToRefresh } from "@/lib/hooks/usePullToRefresh";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Bookmark, Flame, History, Radio, RefreshCw, X, Zap, ZapOff } from "lucide-react";
import { useSavedSearches } from "@/lib/hooks/useSavedSearches";
import { useTrending } from "@/lib/hooks/useTrending";
import { useDownloads } from "@/lib/hooks/useDownloads";
import { useRefreshGuard } from "@/lib/hooks/useRefreshGuard";
import { NewMatchesCard } from "@/components/NewMatchesCard";

export const Route = createFileRoute("/feed/")({
  component: FeedPage,
});

// Cap rendered/queued new matches so a huge listing can't flood the UI or queue.
function FeedPage() {
  const navigate = useNavigate();
  const { settings } = useSettingsStore();
  const [sites, setSites] = useState<SiteInfo[]>(() => mergeSiteLists([], getPluginBrowseSites()));
  const { active, refresh: refreshDownloads } = useDownloads();
  const [playerScene, setPlayerScene] = useState<Scene | null>(null);
  const [playerIndex, setPlayerIndex] = useState(0);

  // --- New matches (saved searches with fresh results) ---
  // Shared with /browse so the two pages cannot drift.
  const {
    saved,
    savedLoading,
    checkingId,
    queueing,
    pollStatus,
    newMatches,
    withNews,
    totalNew,
    refresh: refreshSaved,
    check: handleCheck,
    dismissNewMatches: closeNewMatches,
    queueAllNewMatches,
    pollAll: handlePollAll,
    toggleAutoQueue: handleToggleAutoQueue,
  } = useSavedSearches();

  // --- Trending (capped concurrency + stale-response guard inside the hook) ---
  const trendingEnabled = settings.trending_sites ?? [];
  const {
    trending,
    trendingLoading,
    siteIds: trendSiteIds,
    itemCount: trendItemCount,
    load: loadTrending,
  } = useTrending(trendingEnabled);
  const [trendFilter, setTrendFilter] = useState("all");

  function siteName(id: string): string {
    return sites.find((s) => s.id === id)?.display_name ?? id;
  }

  const visibleTrendIds =
    trendFilter === "all" ? trendSiteIds : trendSiteIds.filter((id) => id === trendFilter);

  // --- Live now (Chaturbate top 8) ---
  const [liveItems, setLiveItems] = useState<MediaItem[]>([]);
  const [liveLoading, setLiveLoading] = useState(false);
  const [liveError, setLiveError] = useState("");

  const loadLive = useCallback(async () => {
    setLiveLoading(true);
    setLiveError("");
    try {
      const page = await api.browse("chaturbate", "livestream", "", 1);
      setLiveItems(page.items.slice(0, 8));
    } catch (e) {
      setLiveItems([]);
      setLiveError(e instanceof Error ? e.message : "Failed to load live streams");
    } finally {
      setLiveLoading(false);
    }
  }, []);

  // Live slugs come from scraped channel/performer names — allowlist them
  // so a value containing "/", "..", or kilobytes of text can't break routing
  // or pollute the browse cache key. Returns "" (navigation no-op) when invalid.
  function liveSlug(item: MediaItem): string {
    const raw = item.channel ?? item.performers[0] ?? "";
    return /^[a-zA-Z0-9_-]{1,64}$/.test(raw) ? raw : "";
  }

  // --- Continue watching ---
  const recent = useRecentlyViewedStore((s) => s.recent);
  const clearRecent = useRecentlyViewedStore((s) => s.clear);
  const [watchMap, setWatchMap] = useState<WatchMap>(new Map());

  const loadWatch = useCallback(async () => {
    try {
      const all = await api.listWatchProgress();
      setWatchMap(toWatchMap(all));
    } catch {
      /* degrade to no overlays */
    }
  }, []);

  function watchState(id: string) {
    return watchFor(watchMap, id);
  }

  function refreshWatch() {
    void api
      .listWatchProgress()
      .then((all) => setWatchMap(toWatchMap(all)))
      .catch(() => {});
  }

  const recentVisible = recent.slice(0, 12);
  function handlePlayRecent(snapshot: Scene) {
    const idx = recentVisible.findIndex((s) => s.id === snapshot.id);
    setPlayerIndex(idx === -1 ? 0 : idx);
    setPlayerScene(snapshot);
  }

  const loadSites = useCallback(async () => {
    try {
      const apiSites = await api.listSites();
      setSites(mergeSiteLists(apiSites, getPluginBrowseSites()));
    } catch {
      setSites(mergeSiteLists([], getPluginBrowseSites()));
    }
  }, []);

  // --- Guarded refresh (P1: no stacked scrape waves from rapid pulls) ---
  const { refresh: handleRefresh } = useRefreshGuard(async () => {
    await Promise.allSettled([
      refreshSaved(),
      loadTrending(),
      loadLive(),
      loadWatch(),
      loadSites(),
      refreshDownloads(),
    ]);
  });

  useEffect(() => {
    void handleRefresh();
  }, [handleRefresh]);

  const { containerRef, pullDistance, refreshing } = usePullToRefresh({
    onRefresh: handleRefresh,
    disabled: false,
  });

  return (
    <div ref={containerRef} className="space-y-6">
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
        <h2 className="text-2xl font-bold">Feed</h2>
        <p className="text-sm text-[var(--color-muted-foreground)]">
          New matches, trending, live, and continue watching
        </p>
      </div>

      {active.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              Active Downloads
              <span className="rounded-full bg-[var(--color-primary)]/20 px-2 py-0.5 text-xs text-[var(--color-primary)]">
                {active.length} Active
              </span>
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            {active.map((job) => (
              <DownloadProgressRow key={job.id} job={job} />
            ))}
          </CardContent>
        </Card>
      )}

      <Card>
        <CardContent className="p-4 space-y-3">
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-2 text-sm font-medium">
              <Bookmark className="h-4 w-4" />
              New matches
            </div>
            <div className="flex items-center gap-2">
              {saved.length > 0 && (
                <span className="text-xs text-[var(--color-muted-foreground)] tabular-nums">
                  {totalNew} new
                </span>
              )}
              {saved.length > 0 && (
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => void handlePollAll()}
                  disabled={checkingId === "all"}
                  title="Check due watchlists now and auto-queue new matches"
                >
                  <RefreshCw
                    className={`h-3.5 w-3.5 ${checkingId === "all" ? "animate-spin" : ""}`}
                  />
                  Check all due
                </Button>
              )}
            </div>
          </div>
          {pollStatus && pollStatus.auto_queue_count > 0 && (
            <p className="text-xs text-[var(--color-muted-foreground)]">
              {pollStatus.last_run
                ? `Last poll ${new Date(pollStatus.last_run.finished_at).toLocaleString()} · ` +
                  `${pollStatus.last_run.checked} checked, ${pollStatus.last_run.queued} queued` +
                  (pollStatus.last_run.errors > 0 ? `, ${pollStatus.last_run.errors} errors` : "") +
                  (pollStatus.due_count > 0 ? ` · ${pollStatus.due_count} due` : "")
                : "Auto-queue on — poller hasn't run yet (first pass within a minute)."}
            </p>
          )}
          {savedLoading && saved.length === 0 ? (
            <SkeletonGrid count={3} cols={1} />
          ) : withNews.length === 0 ? (
            <EmptyState
              icon={<Bookmark className="h-8 w-8" />}
              title="No new matches"
              description="Saved searches with fresh results will show up here. Open any tag, model, channel, or search page and press Save to watch it."
              action={
                <Button asChild size="sm">
                  <Link to="/browse">Browse Sites</Link>
                </Button>
              }
            />
          ) : (
            <div className="grid gap-1">
              {withNews.map((s) => (
                <div
                  key={s.id}
                  className="flex items-center justify-between gap-2 rounded-md px-3 py-2 text-sm hover:bg-[var(--color-muted)] transition"
                >
                  <button
                    type="button"
                    className="flex min-w-0 flex-1 items-center gap-2 text-left"
                    onClick={() => {
                      navigate({
                        to: "/browse/$site/$kind/$slug",
                        params: { site: s.site_id, kind: s.kind, slug: encodeURIComponent(s.slug) },
                      });
                    }}
                  >
                    <span className="min-w-0 flex-1 truncate">
                      <span className="font-medium">{s.name}</span>{" "}
                      <span className="text-xs text-[var(--color-muted-foreground)]">
                        {s.last_checked_at
                          ? `· checked ${new Date(s.last_checked_at).toLocaleString()}`
                          : "· never checked"}
                      </span>
                    </span>
                    <span className="shrink-0 rounded-full bg-[var(--color-primary)] px-2 py-0.5 text-[11px] font-semibold text-[var(--color-primary-foreground)] tabular-nums">
                      {s.new_count} new
                    </span>
                  </button>
                  <div className="flex shrink-0 items-center gap-1">
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => void handleToggleAutoQueue(s)}
                      title={
                        s.auto_queue
                          ? "Auto-queue on: new matches download automatically"
                          : "Auto-queue off: queue new matches automatically"
                      }
                      aria-label={
                        s.auto_queue
                          ? `Disable auto-queue for ${s.name}`
                          : `Enable auto-queue for ${s.name}`
                      }
                      className={s.auto_queue ? "text-[var(--color-primary)]" : ""}
                    >
                      {s.auto_queue ? (
                        <Zap className="h-3.5 w-3.5" />
                      ) : (
                        <ZapOff className="h-3.5 w-3.5" />
                      )}
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => void handleCheck(s.id)}
                      disabled={checkingId === s.id}
                      title="Check for new matches now"
                      aria-label={`Check ${s.name} for new matches now`}
                    >
                      <RefreshCw
                        className={`h-3.5 w-3.5 ${checkingId === s.id ? "animate-spin" : ""}`}
                      />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {newMatches && (
        <NewMatchesCard
          matches={newMatches}
          queueing={queueing}
          onQueueAll={queueAllNewMatches}
          onDismiss={closeNewMatches}
        />
      )}

      <Card>
        <CardContent className="p-4 space-y-3">
          <div className="flex items-center gap-2 text-sm font-medium">
            <Flame className="h-4 w-4 text-orange-500" />
            <span>Trending</span>
          </div>
          {trendingEnabled.length === 0 ? (
            <EmptyState
              icon={<Flame className="h-8 w-8" />}
              title="No trending sites selected"
              description="Pick which sites appear in your feed under Settings."
              action={
                <Button asChild size="sm">
                  <Link to="/settings">Open Settings</Link>
                </Button>
              }
            />
          ) : (
            <>
              {trendSiteIds.length > 1 && (
                <div className="flex gap-1.5 overflow-x-auto pb-1 -mx-1 px-1 snap-x snap-mandatory scrollbar-hide">
                  <button
                    key="all"
                    type="button"
                    onClick={() => setTrendFilter("all")}
                    className={`inline-flex items-center rounded-full px-3 py-1.5 text-xs font-medium transition shrink-0 snap-start ${
                      trendFilter === "all"
                        ? "bg-[var(--color-primary)] text-[var(--color-primary-foreground)]"
                        : "bg-[var(--color-secondary)] hover:bg-[var(--color-muted)]"
                    }`}
                  >
                    All
                  </button>
                  {trendSiteIds.map((id) => (
                    <button
                      key={id}
                      type="button"
                      onClick={() => setTrendFilter(id)}
                      className={`inline-flex items-center rounded-full px-3 py-1.5 text-xs font-medium transition shrink-0 snap-start ${
                        trendFilter === id
                          ? "bg-[var(--color-primary)] text-[var(--color-primary-foreground)]"
                          : "bg-[var(--color-secondary)] hover:bg-[var(--color-muted)]"
                      }`}
                    >
                      {siteName(id)}
                    </button>
                  ))}
                </div>
              )}
              {trendingLoading && trendItemCount === 0 ? (
                <SkeletonGrid count={8} cols={2} />
              ) : trendItemCount === 0 ? (
                <EmptyState
                  icon={<Flame className="h-8 w-8" />}
                  title="No trending content"
                  description="Trending results will appear here once the enabled sites respond."
                />
              ) : (
                <div className="space-y-4">
                  {visibleTrendIds.map((id) => (
                    <div key={id} className="space-y-2">
                      <p className="text-xs font-medium text-[var(--color-muted-foreground)]">
                        {siteName(id)}
                      </p>
                      <div className="grid grid-cols-2 sm:grid-cols-4 lg:grid-cols-5 gap-3">
                        {trending[id].map((item) => (
                          <SceneCard
                            key={item.id}
                            item={item}
                            onDownload={(m) => {
                              void api.queueDownload(m.url, m.site_id);
                            }}
                            onClick={() => {
                              navigate({
                                to: "/browse/$site/$kind/$slug",
                                params: {
                                  site: item.site_id,
                                  kind: "search",
                                  slug: encodeURIComponent(item.title),
                                },
                              });
                            }}
                          />
                        ))}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardContent className="p-4 space-y-3">
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-2 text-sm font-medium">
              <Radio className="h-4 w-4 text-red-500" />
              <span>Live now</span>
            </div>
            <Button asChild size="sm" variant="ghost">
              <Link to="/live">View all</Link>
            </Button>
          </div>
          {liveLoading && liveItems.length === 0 ? (
            <SkeletonGrid count={8} cols={2} />
          ) : liveError && liveItems.length === 0 ? (
            <ErrorState message={liveError} onRetry={() => void loadLive()} />
          ) : liveItems.length === 0 ? (
            <EmptyState
              icon={<Radio className="h-8 w-8" />}
              title="Nobody is live right now"
              description="Live Chaturbate rooms will show up here."
              action={
                <Button asChild size="sm" variant="outline">
                  <Link to="/live">Open Live Streams</Link>
                </Button>
              }
            />
          ) : (
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
              {liveItems.map((item) => (
                <SceneCard
                  key={item.id}
                  item={item}
                  onClick={() => {
                    const slug = liveSlug(item);
                    if (!slug) return;
                    navigate({
                      to: "/live/$site/$slug",
                      params: { site: item.site_id, slug },
                      search: { fromSearch: "0", searchQuery: "" },
                    });
                  }}
                />
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {recentVisible.length > 0 && (
        <div>
          <div className="mb-3 flex items-center justify-between">
            <h3 className="flex items-center gap-2 text-lg font-semibold">
              <History className="h-4 w-4 text-[var(--color-muted-foreground)]" />
              Continue watching
            </h3>
            <button
              type="button"
              onClick={clearRecent}
              className="flex items-center gap-1 text-xs text-[var(--color-muted-foreground)] hover:text-[var(--color-foreground)]"
              title="Clear recently played"
            >
              <X className="h-3 w-3" />
              Clear
            </button>
          </div>
          <div className="grid grid-cols-3 sm:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
            {recentVisible.map((snapshot) => (
              <SceneCard
                key={snapshot.id}
                item={snapshot}
                thumbSrc={sceneThumbUrl(snapshot)}
                watch={watchState(snapshot.id)}
                onClick={(item) => handlePlayRecent(item as Scene)}
              />
            ))}
          </div>
        </div>
      )}

      <ScenePlayerDialog
        scene={playerScene}
        scenes={recentVisible}
        currentIndex={playerIndex}
        open={playerScene !== null}
        onClose={() => {
          setPlayerScene(null);
          refreshWatch();
        }}
        onNavigate={(scene, idx) => {
          setPlayerScene(scene);
          setPlayerIndex(idx);
        }}
      />
    </div>
  );
}
