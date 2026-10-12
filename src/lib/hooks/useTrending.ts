import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "@/lib/api/client";
import type { MediaItem } from "@/lib/types";

/** Max concurrent scrapes per load. Bounded so enabling 10 sites doesn't
 *  fire 10 parallel page fetches (metered data + host rate limiting). */
const TRENDING_CONCURRENCY = 3;
const TRENDING_PER_SITE = 10;
/** Reuse a site's trending row for this long before scraping it again. */
const TRENDING_FRESH_MS = 10 * 60 * 1000;

/** Session cache shared by /browse and /feed, plus in-flight de-duplication so
 *  two loads started together (e.g. Feed's mount refresh and this hook's own
 *  effect) share one request per site. */
const trendingCache = new Map<string, { at: number; items: MediaItem[] }>();
const trendingInflight = new Map<string, Promise<MediaItem[]>>();

/** Drop cached trending rows; the next load scrapes again (pull-to-refresh). */
export function clearTrendingCache(): void {
  trendingCache.clear();
}

function fetchTrendingSite(id: string): Promise<MediaItem[]> {
  const hit = trendingCache.get(id);
  if (hit && Date.now() - hit.at < TRENDING_FRESH_MS) return Promise.resolve(hit.items);
  const pending = trendingInflight.get(id);
  if (pending) return pending;
  const request = api
    .browse(id, "search", "trending", 1)
    .then((page) => {
      const items = page.items.slice(0, TRENDING_PER_SITE);
      trendingCache.set(id, { at: Date.now(), items });
      return items;
    })
    .finally(() => trendingInflight.delete(id));
  trendingInflight.set(id, request);
  return request;
}

export interface UseTrendingResult {
  /** siteId -> items. Only sites that responded are present. */
  trending: Record<string, MediaItem[]>;
  trendingLoading: boolean;
  /** Site ids that returned results, in enabled-site order. */
  siteIds: string[];
  /** Total items across all returned sites. */
  itemCount: number;
  /** Enabled site ids that produced no result at all. */
  failedSiteIds: string[];
  load: () => Promise<void>;
}

/**
 * Trending listings for the sites enabled in Settings, shared by /browse and
 * /feed.
 *
 * Guards (P1 from the Feed review):
 *  - concurrency is capped at {@link TRENDING_CONCURRENCY};
 *  - a monotonically increasing request id discards stale responses, so a fast
 *    second load (e.g. rapid pull-to-refresh) can't be overwritten by a slow
 *    first one;
 *  - results are cached per site for {@link TRENDING_FRESH_MS} and concurrent
 *    loads share requests, so revisiting /browse or /feed doesn't re-scrape;
 *    pull-to-refresh calls {@link clearTrendingCache} first.
 */
export function useTrending(enabledSiteIds: string[]): UseTrendingResult {
  const [trending, setTrending] = useState<Record<string, MediaItem[]>>({});
  const [trendingLoading, setTrendingLoading] = useState(false);
  const [failedSiteIds, setFailedSiteIds] = useState<string[]>([]);
  const requestIdRef = useRef(0);

  // Stable key so callers can pass a freshly-built array each render without
  // retriggering the effect on identity.
  const enabledKey = enabledSiteIds.join(",");

  const load = useCallback(async () => {
    const enabled = enabledKey ? enabledKey.split(",") : [];
    if (enabled.length === 0) {
      setTrending({});
      setTrendingLoading(false);
      setFailedSiteIds([]);
      return;
    }

    const requestId = ++requestIdRef.current;
    setTrendingLoading(true);

    const next: Record<string, MediaItem[]> = {};
    const failed: string[] = [];

    // Worker-pool instead of Promise.allSettled over every site, so at most
    // TRENDING_CONCURRENCY requests are in flight at once.
    const queue = [...enabled];
    const worker = async () => {
      for (;;) {
        const id = queue.shift();
        if (id === undefined) return;
        try {
          next[id] = await fetchTrendingSite(id);
        } catch {
          failed.push(id);
        }
      }
    };
    await Promise.all(
      Array.from({ length: Math.min(TRENDING_CONCURRENCY, enabled.length) }, worker),
    );

    // A newer load started while this one was running — discard these results.
    if (requestId !== requestIdRef.current) return;

    setTrending(next);
    setFailedSiteIds(failed);
    setTrendingLoading(false);
  }, [enabledKey]);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void load();
  }, [load]);

  const siteIds = Object.keys(trending);
  const itemCount = siteIds.reduce((sum, id) => sum + trending[id].length, 0);

  return { trending, trendingLoading, siteIds, itemCount, failedSiteIds, load };
}
