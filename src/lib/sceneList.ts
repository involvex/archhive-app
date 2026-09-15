import type { Scene, SceneSort } from "./types";

/** True when the duration range is contradictory (Q8 guard). */
export function isDurationRangeInvalid(min?: number, max?: number): boolean {
  return min != null && max != null && min > max;
}

/**
 * Apply the search box + sort dropdown on top of backend filter results.
 * Backend `list_scenes_with_filter` has no query/sort params, so when a
 * filter is active the frontend owns query + sort (Q2+Q8 composition).
 * `newest`/`downloaded` preserve backend order (created_at DESC by design).
 */
export function applySceneQueryAndSort(scenes: Scene[], query: string, sort: SceneSort): Scene[] {
  const q = query.trim().toLowerCase();
  const out = q
    ? scenes.filter(
        (s) =>
          s.title.toLowerCase().includes(q) ||
          s.performers.some((p) => p.toLowerCase().includes(q)) ||
          s.tags.some((t) => t.toLowerCase().includes(q)),
      )
    : [...scenes];
  if (sort === "name") out.sort((a, b) => a.title.localeCompare(b.title));
  return out;
}
