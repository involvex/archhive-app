import { create } from "zustand";
import type { BrowseOrientation, MediaItem } from "../types";

export interface BrowseCacheEntry {
  items: MediaItem[];
  page: number;
  hasMore: boolean;
  querySlug: string;
  orientation?: BrowseOrientation;
  categoryKey?: string;
  url?: string;
}

interface BrowseState {
  caches: Record<string, BrowseCacheEntry>;
  get: (key: string) => BrowseCacheEntry | undefined;
  set: (key: string, entry: BrowseCacheEntry) => void;
  patch: (key: string, partial: Partial<BrowseCacheEntry>) => void;
  clear: (key: string) => void;
}

export function browseCacheKey(parts: {
  site: string;
  kind: string;
  slug?: string;
  orientation?: string;
  categoryKey?: string;
  url?: string;
}): string {
  if (parts.url) return `by-url|${parts.url}`;
  if (parts.categoryKey) {
    return `${parts.site}|category|${parts.orientation ?? ""}|${parts.categoryKey}`;
  }
  return `${parts.site}|${parts.kind}|${parts.slug ?? ""}|${parts.orientation ?? ""}`;
}

export const useBrowseStore = create<BrowseState>((set, get) => ({
  caches: {},
  get: (key) => get().caches[key],
  set: (key, entry) =>
    set((s) => ({
      caches: { ...s.caches, [key]: entry },
    })),
  patch: (key, partial) =>
    set((s) => {
      const prev = s.caches[key];
      if (!prev) return s;
      return { caches: { ...s.caches, [key]: { ...prev, ...partial } } };
    }),
  clear: (key) =>
    set((s) => {
      const next = { ...s.caches };
      delete next[key];
      return { caches: next };
    }),
}));
