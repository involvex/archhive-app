import type { BuilderQuery } from "@/components/QueryBuilder";

/** A saved Online library query on a scope (saved search id, "@bookmarks",
 * or "" for everything): listed like a saved search, re-run live so new
 * results from updates show up. Kept on this device. */
export interface OnlineView {
  id: string;
  name: string;
  scope: string;
  query: BuilderQuery;
  /** Length filter id (Online library DURATIONS), "" = any. */
  duration?: string;
}

const KEY = "online-views";

export function loadViews(): OnlineView[] {
  try {
    const raw = localStorage.getItem(KEY);
    const list = raw ? (JSON.parse(raw) as OnlineView[]) : [];
    return Array.isArray(list) ? list : [];
  } catch {
    return [];
  }
}

export function saveViews(views: OnlineView[]): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(views));
  } catch {
    // storage unavailable: views last until the app restarts
  }
}
