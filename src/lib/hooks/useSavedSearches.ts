import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "@/lib/api/client";
import type { MediaItem, SavedSearch, WatchlistStatus } from "@/lib/types";

/**
 * Cap rendered/queued new matches so a huge listing can't flood the UI or
 * queue. Shared by /browse and /feed — keep the single source of truth here.
 */
export const MAX_NEW_MATCHES = 25;

export interface NewMatches {
  searchId: string;
  name: string;
  items: MediaItem[];
  total: number;
}

/** Only queue absolute http(s) URLs — rejects anything a scrape could poison. */
export function queueableUrls(items: MediaItem[]): string[] {
  return items.map((i) => i.url.trim()).filter((u) => /^https?:\/\//i.test(u));
}

export interface UseSavedSearchesResult {
  saved: SavedSearch[];
  savedLoading: boolean;
  checkingId: string | null;
  queueing: boolean;
  pollStatus: WatchlistStatus | null;
  newMatches: NewMatches | null;
  /** Saved searches that have at least one fresh result. */
  withNews: SavedSearch[];
  /** Sum of new_count across all saved searches. */
  totalNew: number;
  refresh: () => Promise<void>;
  check: (id: string) => Promise<void>;
  dismissNewMatches: () => Promise<void>;
  queueAllNewMatches: () => Promise<void>;
  pollAll: () => Promise<void>;
  toggleAutoQueue: (s: SavedSearch) => Promise<void>;
  clearNewMatches: () => void;
}

/**
 * Saved-search watchlist state shared by /browse and /feed so the two pages
 * cannot drift. Extracted to avoid duplicating the five handlers.
 */
export function useSavedSearches(): UseSavedSearchesResult {
  const [saved, setSaved] = useState<SavedSearch[]>([]);
  const savedRef = useRef<SavedSearch[]>([]);
  const [checkingId, setCheckingId] = useState<string | null>(null);
  const [queueing, setQueueing] = useState(false);
  const [pollStatus, setPollStatus] = useState<WatchlistStatus | null>(null);
  const [newMatches, setNewMatches] = useState<NewMatches | null>(null);
  const [savedLoading, setSavedLoading] = useState(false);

  useEffect(() => {
    savedRef.current = saved;
  }, [saved]);

  const refresh = useCallback(async () => {
    setSavedLoading(true);
    const [savedRes, pollRes] = await Promise.allSettled([
      api.listSavedSearches(),
      api.watchlistStatus(),
    ]);
    if (savedRes.status === "fulfilled") setSaved(savedRes.value);
    else setSaved([]);
    if (pollRes.status === "fulfilled") setPollStatus(pollRes.value);
    else setPollStatus(null);
    setSavedLoading(false);
  }, []);

  const check = useCallback(
    async (id: string) => {
      setCheckingId(id);
      try {
        const result = await api.checkSavedSearch(id);
        await refresh();
        if (result.new_count > 0) {
          const search = savedRef.current.find((s) => s.id === id);
          setNewMatches({
            searchId: id,
            name: search?.name ?? "Saved search",
            items: result.new_items.slice(0, MAX_NEW_MATCHES),
            total: result.new_items.length,
          });
        }
      } catch (e) {
        console.error(e);
      } finally {
        setCheckingId(null);
      }
    },
    [refresh],
  );

  const clearNewMatches = useCallback(() => setNewMatches(null), []);

  const dismissNewMatches = useCallback(async () => {
    if (!newMatches) return;
    try {
      await api.dismissSavedSearchNews(newMatches.searchId);
    } catch (e) {
      console.error(e);
    }
    setNewMatches(null);
    void refresh();
  }, [newMatches, refresh]);

  const queueAllNewMatches = useCallback(async () => {
    if (!newMatches || queueing) return;
    const urls = queueableUrls(newMatches.items);
    if (urls.length === 0) {
      setNewMatches(null);
      return;
    }
    setQueueing(true);
    try {
      await api.queueDownloads(urls);
      setNewMatches(null);
      void refresh();
    } catch (e) {
      console.error(e);
    } finally {
      setQueueing(false);
    }
  }, [newMatches, queueing, refresh]);

  const pollAll = useCallback(async () => {
    setCheckingId("all");
    try {
      await api.pollWatchlist();
      await refresh();
    } catch (e) {
      console.error(e);
    } finally {
      setCheckingId(null);
    }
  }, [refresh]);

  const toggleAutoQueue = useCallback(async (s: SavedSearch) => {
    try {
      await api.setSavedSearchAutoQueue(s.id, !s.auto_queue);
      setSaved((prev) =>
        prev.map((p) => (p.id === s.id ? { ...p, auto_queue: !s.auto_queue } : p)),
      );
    } catch (e) {
      console.error(e);
    }
  }, []);

  return {
    saved,
    savedLoading,
    checkingId,
    queueing,
    pollStatus,
    newMatches,
    withNews: saved.filter((s) => s.new_count > 0),
    totalNew: saved.reduce((sum, s) => sum + s.new_count, 0),
    refresh,
    check,
    dismissNewMatches,
    queueAllNewMatches,
    pollAll,
    toggleAutoQueue,
    clearNewMatches,
  };
}
