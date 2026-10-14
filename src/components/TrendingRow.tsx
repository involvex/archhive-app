import { useState } from "react";
import { toast } from "react-hot-toast";
import { api } from "@/lib/api/client";
import type { MediaItem } from "@/lib/types";
import { SceneCard } from "@/components/SceneCard";
import { UrlPlayerDialog } from "@/components/UrlPlayerDialog";
import { BrowseItemDetailsDialog } from "@/components/BrowseItemDetailsDialog";

/**
 * One horizontally scrolling row of trending videos (Browse and Feed).
 * Compact cards with small Info / Download icons; tapping a card opens the
 * player (tags, Bookmark), with Previous / Next within the row.
 */
export function TrendingRow({ items }: { items: MediaItem[] }) {
  const [watchItem, setWatchItem] = useState<MediaItem | null>(null);
  const [infoItem, setInfoItem] = useState<MediaItem | null>(null);
  if (items.length === 0) return null;
  return (
    <>
      <div className="flex gap-3 overflow-x-auto pb-2 -mx-1 px-1 snap-x snap-mandatory scrollbar-hide">
        {items.map((item) => (
          <div key={item.id} className="shrink-0 w-44 snap-start">
            <SceneCard
              item={item}
              listView={false}
              compactActions
              onClick={() => setWatchItem(item)}
              onWatch={setWatchItem}
              onInfo={setInfoItem}
              onDownload={(m) =>
                void api
                  .queueDownload(m.url, m.site_id, m.title)
                  .then(() => toast.success("Added to downloads"))
                  .catch((e: unknown) => toast.error(String(e)))
              }
            />
          </div>
        ))}
      </div>
      <UrlPlayerDialog
        item={watchItem}
        playlist={items}
        open={watchItem !== null}
        onClose={() => setWatchItem(null)}
        onSelectItem={setWatchItem}
      />
      <BrowseItemDetailsDialog
        item={infoItem}
        open={infoItem !== null}
        onClose={() => setInfoItem(null)}
      />
    </>
  );
}
