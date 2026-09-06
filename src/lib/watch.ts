import type { CardWatchState } from "@/components/SceneCard";
import type { WatchProgress } from "./types";

/** Scene id → progress map, as returned by `api.listWatchProgress`. */
export type WatchMap = Map<string, WatchProgress>;

/** Shared #26 mapper: watch row → card overlay state (review: was duplicated). */
export function watchFor(watchMap: WatchMap, id: string): CardWatchState | null {
  const w = watchMap.get(id);
  if (!w) return null;
  return { position: w.position_secs, duration: w.duration_secs, watched: w.watched };
}

/** Build a WatchMap from a progress list. */
export function toWatchMap(all: WatchProgress[]): WatchMap {
  return new Map(all.map((w) => [w.scene_id, w]));
}
