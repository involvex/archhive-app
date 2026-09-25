import { useState } from "react";
import { History, X } from "lucide-react";
import { SceneCard } from "@/components/SceneCard";
import { ScenePlayerDialog } from "@/components/ScenePlayerDialog";
import { sceneThumbUrl, isVideoScene } from "@/lib/mediaUrl";
import { useRecentlyViewedStore } from "@/lib/stores/recentlyViewed";
import { watchFor, type WatchMap } from "@/lib/watch";
import type { Scene } from "@/lib/types";

interface ContinueWatchingRailProps {
  /** Live library scenes — used to prune stale snapshots and to play from. */
  scenes: Scene[];
  watchMap: WatchMap;
  max?: number;
}

/**
 * Shared "Continue watching" rail (T1.1). Backed by the persisted
 * `useRecentlyViewedStore` (written by ScenePlayerDialog on play); no backend
 * involved. Prunes entries whose scene left the library instead of opening a
 * player that 404s on getScene.
 */
export function ContinueWatchingRail({ scenes, watchMap, max = 12 }: ContinueWatchingRailProps) {
  const recent = useRecentlyViewedStore((s) => s.recent);
  const clearRecent = useRecentlyViewedStore((s) => s.clear);
  const removeRecent = useRecentlyViewedStore((s) => s.remove);
  const [playerScene, setPlayerScene] = useState<Scene | null>(null);
  const [playerIndex, setPlayerIndex] = useState(0);

  const videoScenes = scenes.filter(isVideoScene);
  const visible = recent.slice(0, max);
  if (visible.length === 0) return null;

  function handlePlay(scene: Scene) {
    const idx = videoScenes.findIndex((s) => s.id === scene.id);
    if (idx === -1) return;
    setPlayerIndex(idx);
    setPlayerScene(scene);
  }

  function handlePlaySnapshot(snapshot: Scene) {
    const live = videoScenes.find((s) => s.id === snapshot.id);
    if (!live) {
      if (scenes.length > 0) removeRecent(snapshot.id);
      return;
    }
    if (!isVideoScene(live)) {
      setPlayerScene(live);
      setPlayerIndex(0);
      return;
    }
    handlePlay(live);
  }

  return (
    <div>
      <div className="mb-3 flex items-center justify-between">
        <h3 className="flex items-center gap-2 text-lg font-semibold">
          <History className="h-4 w-4 text-[var(--color-muted-foreground)]" />
          Continue watching
        </h3>
        <button
          type="button"
          onClick={clearRecent}
          className="flex items-center gap-1 text-xs text-[var(--color-muted-foreground)] hover:text-[var(--color-foreground)]"
          title="Clear recently played"
        >
          <X className="h-3 w-3" />
          Clear
        </button>
      </div>
      <div className="grid grid-cols-3 sm:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
        {visible.map((snapshot) => (
          <SceneCard
            key={snapshot.id}
            item={snapshot}
            thumbSrc={sceneThumbUrl(snapshot)}
            watch={watchFor(watchMap, snapshot.id)}
            onClick={(item) => handlePlaySnapshot(item as Scene)}
          />
        ))}
      </div>
      <ScenePlayerDialog
        scene={playerScene}
        scenes={videoScenes}
        currentIndex={playerIndex}
        open={playerScene !== null}
        onClose={() => setPlayerScene(null)}
        onNavigate={(scene, idx) => {
          setPlayerScene(scene);
          setPlayerIndex(idx);
        }}
      />
    </div>
  );
}
