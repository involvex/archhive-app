import type { BrowseFilters, BrowseOrientation } from "@/lib/types";

const ORIENTATIONS: readonly BrowseOrientation[] = ["straight", "gay", "lesbian", "transgender"];
const FILTER_KEYS = [
  "sort",
  "period",
  "duration",
  "quality",
  "include",
  "exclude",
  "tag",
  "source",
] as const;

/** URL search params of a browse page: orientation plus site-side filters. */
export type BrowseSearch = { orientation?: BrowseOrientation } & BrowseFilters;

/** Validate untrusted search params (saved-search links, deep links). */
export function parseBrowseSearch(search: Record<string, unknown>): BrowseSearch {
  const out: BrowseSearch = {};
  const o = search.orientation;
  if (typeof o === "string" && (ORIENTATIONS as readonly string[]).includes(o)) {
    out.orientation = o as BrowseOrientation;
  }
  for (const key of FILTER_KEYS) {
    // The router parses search values as JSON, so numeric ids ("35") arrive
    // as numbers. Option ids / slug lists only; the backend re-validates.
    const raw = search[key];
    const v = typeof raw === "number" && Number.isFinite(raw) ? String(raw) : raw;
    if (typeof v === "string" && v.trim() && v.length <= 300) out[key] = v.trim();
  }
  return out;
}

/** The filter part of browse search params. */
export function filtersFromSearch(search: BrowseSearch): BrowseFilters {
  const out: BrowseFilters = {};
  for (const key of FILTER_KEYS) {
    if (search[key]) out[key] = search[key];
  }
  return out;
}
