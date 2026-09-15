import type { Performer } from "./types";

export type PerformerSort = "name" | "scenes";

const STORAGE_KEY = "archhive.performerSort";

export function loadPerformerSort(): PerformerSort {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw === "scenes" ? "scenes" : "name";
  } catch {
    return "name";
  }
}

export function savePerformerSort(sort: PerformerSort): void {
  try {
    localStorage.setItem(STORAGE_KEY, sort);
  } catch {
    /* private mode / quota — sort just won't persist */
  }
}

/** Client-side ordering for the performers grid (Q15). */
export function sortPerformers(performers: Performer[], sort: PerformerSort): Performer[] {
  return [...performers].sort((a, b) =>
    sort === "scenes"
      ? b.scene_count - a.scene_count || a.name.localeCompare(b.name)
      : a.name.localeCompare(b.name),
  );
}
