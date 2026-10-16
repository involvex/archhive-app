import { Link, createFileRoute, useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useState } from "react";
import { api } from "@/lib/api/client";
import { useSettingsStore } from "@/lib/stores/settings";
import type { MediaItem, Scene } from "@/lib/types";
import { toWatchMap, watchFor, type WatchMap } from "@/lib/watch";
import { sceneThumbUrl, isVideoScene } from "@/lib/mediaUrl";
import { buildAffinity, rankMediaItems, recommendScenes, resolveRecPrefs } from "@/lib/recommend";
import { useSavedSearches } from "@/lib/hooks/useSavedSearches";
import { useTrending } from "@/lib/hooks/useTrending";
import { SceneCard } from "@/components/SceneCard";
import { ScenePlayerDialog } from "@/components/ScenePlayerDialog";
import { SkeletonGrid } from "@/components/SkeletonGrid";
import { ErrorState } from "@/components/ErrorState";
import { EmptyState } from "@/components/EmptyState";
import { Button } from "@/components/ui/button";
import { Compass, Radio, Settings2, Sparkles, Zap } from "lucide-react";

export const Route = createFileRoute("/recommendations/")({
  component: RecommendationsPage,
});

function RecommendationsPage() {
  const navigate = useNavigate();
  const { settings } = useSettingsStore();
  const [scenes, setScenes] = useState<Scene[]>([]);
  const [watchMap, setWatchMap] = useState<WatchMap>(new Map());
  const [loadError, setLoadError] = useState("");
  const [loading, setLoading] = useState(true);
  const [playerScene, setPlayerScene] = useState<Scene | null>(null);
  const [playerIndex, setPlayerIndex] = useState(0);

  const prefs = useMemo(() => resolveRecPrefs(settings), [settings]);
  const { newMatches, refresh: refreshSaved } = useSavedSearches();
  const {
    trending,
    trendingLoading,
    load: loadTrending,
  } = useTrending(settings.trending_sites ?? []);

  const [liveItems, setLiveItems] = useState<MediaItem[]>([]);
  const [liveLoading, setLiveLoading] = useState(false);

  const loadLive = useCallback(async () => {
    setLiveLoading(true);
    try {
      const page = await api.browse("chaturbate", "livestream", "", 1);
      setLiveItems(page.items.slice(0, 24));
    } catch {
      setLiveItems([]);
    } finally {
      setLiveLoading(false);
    }
  }, []);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setLoading(true);
    setLoadError("");
    const scenesP = api
      .listScenes()
      .then(setScenes)
      .catch((e) => setLoadError(e instanceof Error ? e.message : "Failed to load scenes"));
    const watchP = api
      .listWatchProgress()
      .then((all) => setWatchMap(toWatchMap(all)))
      .catch(() => {});
    Promise.allSettled([scenesP, watchP]).finally(() => setLoading(false));
  }, []);

  // Phase 2 sources load alongside; sections stay hidden until ranked.
  useEffect(() => {
    if (!prefs.enabled) return;
    // eslint-disable-next-line react-hooks/set-state-in-effect
    if (prefs.includeLive) void loadLive();
    if (prefs.includeFeed) {
      void refreshSaved();
      void loadTrending();
    }
  }, [prefs.enabled, prefs.includeLive, prefs.includeFeed, loadLive, refreshSaved, loadTrending]);

  // Live slugs come from scraped channel/performer names — allowlist them
  // (same rule as /feed) so hostile values can't break routing.
  function liveSlug(item: MediaItem): string {
    const raw = item.channel ?? item.performers[0] ?? "";
    return /^[a-zA-Z0-9_-]{1,64}$/.test(raw) ? raw : "";
  }
  const recommendations = useMemo(
    () => recommendScenes(scenes, [...watchMap.values()], prefs),
    [scenes, watchMap, prefs],
  );
  const affinity = useMemo(() => buildAffinity(scenes, [...watchMap.values()]), [scenes, watchMap]);
  const livePicks = useMemo(
    () => (prefs.includeLive ? rankMediaItems(liveItems, affinity, prefs, 8) : []),
    [liveItems, affinity, prefs],
  );
  const freshPicks = useMemo(() => {
    if (!prefs.includeFeed) return [];
    const pool = [...(newMatches?.items ?? []), ...Object.values(trending).flat()];
    return rankMediaItems(pool, affinity, prefs, 12);
  }, [newMatches, trending, affinity, prefs]);
  const videoScenes = useMemo(() => scenes.filter(isVideoScene), [scenes]);

  function handlePlay(scene: Scene) {
    const idx = videoScenes.findIndex((s) => s.id === scene.id);
    if (idx === -1) return;
    setPlayerIndex(idx);
    setPlayerScene(scene);
  }

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between gap-3">
        <div>
          <h2 className="flex items-center gap-2 text-2xl font-bold">
            <Sparkles className="h-5 w-5 text-[var(--color-primary)]" />
            Recommended for you
          </h2>
          <p className="text-sm text-[var(--color-muted-foreground)]">
            Unwatched library scenes scored from your watch history and preferences
          </p>
        </div>
        <Button asChild size="sm" variant="outline">
          <Link to="/settings">
            <Settings2 className="h-4 w-4 mr-2" />
            Tune
          </Link>
        </Button>
      </div>

      {loadError && <ErrorState message={loadError} onRetry={() => window.location.reload()} />}

      {!loadError && !loading && !prefs.enabled && (
        <EmptyState
          icon={<Sparkles className="h-8 w-8" />}
          title="Recommendations are off"
          description="Enable them in Settings to see picks on your Dashboard."
          action={
            <Button asChild size="sm">
              <Link to="/settings">Open Settings</Link>
            </Button>
          }
        />
      )}

      {!loadError &&
        !loading &&
        !liveLoading &&
        !trendingLoading &&
        prefs.enabled &&
        recommendations.length === 0 &&
        livePicks.length === 0 &&
        freshPicks.length === 0 && (
          <EmptyState
            icon={<Sparkles className="h-8 w-8" />}
            title="Not enough signal yet"
            description="Watch some scenes — or set preferred tags and performers in Settings — and picks will show up here."
            action={
              scenes.length === 0 ? (
                <Button asChild size="sm">
                  <Link to="/browse">
                    <Compass className="h-4 w-4 mr-2" />
                    Browse Sites
                  </Link>
                </Button>
              ) : (
                <Button asChild size="sm">
                  <Link to="/settings">Tune preferences</Link>
                </Button>
              )
            }
          />
        )}

      {!loadError && (loading || recommendations.length > 0) && (
        <>
          {loading && scenes.length === 0 ? (
            <SkeletonGrid count={12} cols={6} />
          ) : (
            <div className="grid grid-cols-3 sm:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
              {recommendations.map(({ scene, reasons }) => (
                <div key={scene.id}>
                  <SceneCard
                    item={scene}
                    thumbSrc={sceneThumbUrl(scene)}
                    watch={watchFor(watchMap, scene.id)}
                    onClick={(item) => handlePlay(item as Scene)}
                  />
                  {reasons.length > 0 && (
                    <p
                      className="mt-1 truncate text-[11px] text-[var(--color-muted-foreground)]"
                      title={reasons.join(" · ")}
                    >
                      {reasons.join(" · ")}
                    </p>
                  )}
                </div>
              ))}
            </div>
          )}
        </>
      )}

      {prefs.enabled && !loading && prefs.includeLive && livePicks.length > 0 && (
        <div>
          <div className="mb-3 flex items-center justify-between">
            <h3 className="flex items-center gap-2 text-lg font-semibold">
              <Radio className="h-4 w-4 text-red-500" />
              Live picks
            </h3>
            <Link
              to="/live"
              className="text-xs text-[var(--color-muted-foreground)] hover:text-[var(--color-foreground)]"
            >
              View all
            </Link>
          </div>
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
            {livePicks.map(({ item, reasons }) => (
              <div key={item.id}>
                <SceneCard
                  item={item}
                  onClick={() => {
                    const slug = liveSlug(item);
                    if (!slug) return;
                    void navigate({
                      to: "/live/$site/$slug",
                      params: { site: item.site_id, slug },
                      search: { fromSearch: "0", searchQuery: "" },
                    });
                  }}
                />
                {reasons.length > 0 && (
                  <p
                    className="mt-1 truncate text-[11px] text-[var(--color-muted-foreground)]"
                    title={reasons.join(" · ")}
                  >
                    {reasons.join(" · ")}
                  </p>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {prefs.enabled &&
        !loading &&
        prefs.includeFeed &&
        (freshPicks.length > 0 || trendingLoading) && (
          <div>
            <div className="mb-3 flex items-center justify-between">
              <h3 className="flex items-center gap-2 text-lg font-semibold">
                <Zap className="h-4 w-4 text-[var(--color-primary)]" />
                Fresh matches for you
              </h3>
              <Link
                to="/feed"
                className="text-xs text-[var(--color-muted-foreground)] hover:text-[var(--color-foreground)]"
              >
                View all
              </Link>
            </div>
            {trendingLoading && freshPicks.length === 0 ? (
              <SkeletonGrid count={8} cols={4} />
            ) : (
              <div className="grid grid-cols-2 sm:grid-cols-4 lg:grid-cols-5 gap-3">
                {freshPicks.map(({ item, reasons }) => (
                  <div key={item.id}>
                    <SceneCard
                      item={item}
                      onDownload={(m) => {
                        void api.queueDownload(m.url, m.site_id);
                      }}
                      onClick={() => {
                        void navigate({
                          to: "/browse/$site/$kind/$slug",
                          params: {
                            site: item.site_id,
                            kind: "search",
                            slug: encodeURIComponent(item.title),
                          },
                        });
                      }}
                    />
                    {reasons.length > 0 && (
                      <p
                        className="mt-1 truncate text-[11px] text-[var(--color-muted-foreground)]"
                        title={reasons.join(" · ")}
                      >
                        {reasons.join(" · ")}
                      </p>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

      <ScenePlayerDialog
        scene={playerScene}
        scenes={videoScenes}
        currentIndex={playerIndex}
        open={playerScene !== null}
        onClose={() => {
          setPlayerScene(null);
          void api
            .listWatchProgress()
            .then((all) => setWatchMap(toWatchMap(all)))
            .catch(() => {});
        }}
        onNavigate={(scene, idx) => {
          setPlayerScene(scene);
          setPlayerIndex(idx);
        }}
      />
    </div>
  );
}
