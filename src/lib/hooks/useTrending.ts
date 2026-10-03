import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "@/lib/api/client";
import type { MediaItem } from "@/lib/types";

/** Max concurrent scrapes per load. Bounded so enabling 10 sites doesn't
 *  fire 10 parallel page fetches (metered data + host rate limiting). */
const TRENDING_CONCURRENCY = 3;
const TRENDING_PER_SITE = 10;

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
 *  - results are memoised per site for the session via the browse cache, so
 *    repeated refreshes don't re-scrape unless the cache is stale.
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
          const page = await api.browse(id, "search", "trending", 1);
          next[id] = page.items.slice(0, TRENDING_PER_SITE);
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
