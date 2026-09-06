import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { Scene } from "../types";

const MAX_RECENT = 12;

interface RecentlyViewedState {
  recent: Scene[];
  record: (scene: Scene) => void;
  remove: (id: string) => void;
  clear: () => void;
}

/**
 * Session-persistent "Continue watching" rail (Q11).
 * Records scenes as they are opened in the player; no backend involved.
 * Full watch-history with resume positions is tracked separately (#26).
 */
export const useRecentlyViewedStore = create<RecentlyViewedState>()(
  persist(
    (set) => ({
      recent: [],
      record: (scene) =>
        set((s) => ({
          recent: [scene, ...s.recent.filter((r) => r.id !== scene.id)].slice(0, MAX_RECENT),
        })),
      remove: (id) => set((s) => ({ recent: s.recent.filter((r) => r.id !== id) })),
      clear: () => set({ recent: [] }),
    }),
    { name: "archhive-recently-viewed" },
  ),
);
