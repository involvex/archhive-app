import { useCallback, useRef, useState } from "react";
import type { MediaItem, Scene } from "@/lib/types";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Check, Clock, Download, Info, Play, Pencil, MoreVertical } from "lucide-react";

export interface CardWatchState {
  position: number;
  duration: number;
  watched: boolean;
}

type CardItem = MediaItem | Scene;

interface SceneCardProps {
  item: CardItem;
  onDownload?: (item: MediaItem) => void;
  onInfo?: (item: MediaItem) => void;
  onWatch?: (item: MediaItem) => void;
  onEdit?: (item: Scene) => void;
  onContextMenu?: (item: CardItem, x: number, y: number) => void;
  selected?: boolean;
  selectionMode?: boolean;
  thumbSrc?: string;
  onClick?: (item: CardItem) => void;
  listView?: boolean;
  /** #26 watch state — progress bar overlay + watched chip (scenes only). */
  watch?: CardWatchState | null;
}

function isMediaItem(item: CardItem): item is MediaItem {
  return "site_id" in item;
}

function getItemTitle(item: CardItem): string {
  return item.title;
}

function getItemPerformers(item: CardItem): string[] {
  return item.performers;
}

function getItemChannel(item: CardItem): string | undefined {
  return item.channel;
}

function getItemTags(item: CardItem): string[] {
  return item.tags;
}

function getItemDuration(item: CardItem): number | undefined {
  return item.duration;
}

function getItemThumb(i: CardItem, thumbSrc?: string): string | undefined {
  if (thumbSrc) return thumbSrc;
  if (isMediaItem(i)) return i.thumbnail;
  return (i as Scene).thumb;
}

function getItemDescription(item: CardItem): string | undefined {
  if (isMediaItem(item)) return item.description;
  return undefined;
}

function getItemFileSize(item: CardItem): number | undefined {
  if (isMediaItem(item)) return undefined;
  return (item as Scene).file_size;
}

// Q16: resolution badge (WxH) when the backend has probed it.
function getItemResolution(item: CardItem): string | undefined {
  if (isMediaItem(item)) return undefined;
  const scene = item as Scene;
  if (scene.width != null && scene.height != null && scene.width > 0 && scene.height > 0) {
    return `${scene.width}×${scene.height}`;
  }
  return undefined;
}

export function SceneCard({
  item,
  onDownload,
  onInfo,
  onWatch,
  onEdit,
  onContextMenu,
  selected = false,
  selectionMode = false,
  thumbSrc,
  onClick,
  listView = false,
  watch,
}: SceneCardProps) {
  const title = getItemTitle(item);
  const performers = getItemPerformers(item);
  const channel = getItemChannel(item);
  const tags = getItemTags(item);
  const duration = getItemDuration(item);
  const fileSize = getItemFileSize(item);
  const resolution = getItemResolution(item);
  // #26: fraction watched (0..1) for the overlay bar; hidden when no data.
  const watchFraction =
    watch && !isMediaItem(item) && watch.duration > 0
      ? Math.min(1, Math.max(0, watch.position / watch.duration))
      : null;
  const showWatched = Boolean(watch?.watched) && !isMediaItem(item);
  const thumb = getItemThumb(item, thumbSrc);
  const description = getItemDescription(item);

  const [swipeX, setSwipeX] = useState(0);
  const [swiping, setSwiping] = useState(false);
  const pointerStartX = useRef(0);
  const pointerStartY = useRef(0);
  const isTouch = useRef(false);
  const swipeResetTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const hasActions = Boolean(onInfo || onDownload || onWatch || onEdit);
  const showOverlay = !selectionMode;

  const resetSwipe = useCallback(() => {
    setSwipeX(0);
    setSwiping(false);
  }, []);

  const onPointerDown = useCallback(
    (e: React.PointerEvent) => {
      if (selectionMode || !showOverlay) return;
      if (swipeResetTimer.current) clearTimeout(swipeResetTimer.current);
      isTouch.current = e.pointerType === "touch";
      pointerStartX.current = e.clientX;
      pointerStartY.current = e.clientY;
    },
    [selectionMode, showOverlay],
  );

  const onPointerMove = useCallback((e: React.PointerEvent) => {
    if (!isTouch.current) return;
    const dx = e.clientX - pointerStartX.current;
    const dy = e.clientY - pointerStartY.current;
    if (Math.abs(dy) > Math.abs(dx) * 0.6) return;
    setSwiping(true);
    setSwipeX(Math.max(-120, Math.min(120, dx)));
  }, []);

  const onPointerUp = useCallback(
    (e: React.PointerEvent) => {
      if (!swiping || !(e.target as HTMLElement).closest("[data-card-root]")) {
        resetSwipe();
        return;
      }
      if (Math.abs(swipeX) > 60) {
        if (swipeX < -60 && onDownload) {
          void Promise.resolve().then(() => onDownload(item as MediaItem));
        } else if (swipeX > 60 && onWatch) {
          void Promise.resolve().then(() => onWatch(item as MediaItem));
        }
      }
      resetSwipe();
    },
    [swipeX, swiping, onDownload, onWatch, item, resetSwipe],
  );

  if (listView) {
    return (
      <Card
        data-card-root
        className="group relative flex overflow-hidden transition-all duration-150 cursor-pointer hover:border-[var(--color-primary)] active:scale-[0.99]"
        onContextMenu={(e) => {
          e.preventDefault();
          if (onContextMenu) onContextMenu(item, e.clientX, e.clientY);
        }}
        onClick={(e) => {
          if (selectionMode || (e.target as HTMLElement).closest("button")) return;
          if (onClick) onClick(item);
        }}
      >
        <div className="w-40 shrink-0 aspect-video bg-[var(--color-muted)] relative overflow-hidden">
          {thumb ? (
            <img
              src={thumb}
              alt={title}
              className="h-full w-full object-cover transition-transform duration-300 group-hover:scale-105"
              loading="lazy"
              onError={(e) => {
                (e.target as HTMLImageElement).style.display = "none";
              }}
            />
          ) : (
            <div className="flex h-full items-center justify-center text-[var(--color-muted-foreground)] text-xs">
              No preview
            </div>
          )}
          {duration != null && duration > 0 && (
            <span className="absolute bottom-1 right-1 rounded bg-black/70 px-1.5 py-0.5 text-xs flex items-center gap-1">
              <Clock className="h-3 w-3" />
              {formatDuration(duration)}
            </span>
          )}
          {resolution && (
            <span className="absolute bottom-1 left-1 rounded bg-black/70 px-1.5 py-0.5 text-[10px] tabular-nums">
              {resolution}
            </span>
          )}
          {showWatched && (
            <span className="absolute top-1 left-1 flex items-center gap-1 rounded bg-green-600/90 px-1.5 py-0.5 text-[10px] font-medium text-white">
              <Check className="h-3 w-3" />
              Watched
            </span>
          )}
          {watchFraction != null && watchFraction > 0 && (
            <div className="absolute inset-x-0 bottom-0 h-1 bg-black/60">
              <div
                className="h-full bg-red-500"
                style={{ width: `${Math.round(watchFraction * 100)}%` }}
              />
            </div>
          )}
        </div>

        <CardContent className="flex flex-1 flex-col justify-between p-3 min-w-0">
          <div className="space-y-1 min-w-0">
            <p className="line-clamp-2 text-sm font-medium leading-tight">{title}</p>
            {(channel || performers.length > 0) && (
              <p className="text-[11px] text-[var(--color-muted-foreground)] truncate">
                {channel || performers.join(", ")}
              </p>
            )}
            {description && (
              <p className="text-[11px] text-[var(--color-muted-foreground)] line-clamp-2">
                {description}
              </p>
            )}
          </div>

          <div className="flex flex-wrap items-center gap-1.5 mt-2">
            {tags.slice(0, 4).map((tag) => (
              <span
                key={tag}
                className="rounded bg-[var(--color-secondary)] px-1.5 py-0.5 text-[10px]"
              >
                {tag}
              </span>
            ))}
            {hasActions && (
              <div className="flex items-center gap-1 ml-auto">
                {onInfo && (
                  <Button
                    size="sm"
                    variant="outline"
                    className="h-7 px-2 text-xs"
                    onClick={(e) => {
                      e.stopPropagation();
                      onInfo(item as MediaItem);
                    }}
                    aria-label="Info"
                  >
                    <Info className="h-3 w-3" />
                  </Button>
                )}
                {onEdit && (
                  <Button
                    size="sm"
                    variant="outline"
                    className="h-7 px-2 text-xs"
                    onClick={(e) => {
                      e.stopPropagation();
                      onEdit(item as Scene);
                    }}
                    aria-label="Edit"
                  >
                    <Pencil className="h-3 w-3" />
                  </Button>
                )}
                {onDownload && (
                  <Button
                    size="sm"
                    variant={onWatch ? "outline" : "default"}
                    className="h-7 px-2 text-xs"
                    onClick={(e) => {
                      e.stopPropagation();
                      onDownload(item as MediaItem);
                    }}
                    aria-label="Download"
                  >
                    <Download className="h-3 w-3" />
                  </Button>
                )}
              </div>
            )}
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card
      data-card-root
      className={`group relative overflow-hidden transition-all duration-150 cursor-pointer hover:border-[var(--color-primary)] active:scale-[0.98] ${
        selected ? "ring-2 ring-[var(--color-primary)]" : ""
      }`}
      style={
        swipeX !== 0
          ? { transform: `translateX(${swipeX}px)`, transition: "transform 0.1s ease" }
          : undefined
      }
      onContextMenu={(e) => {
        e.preventDefault();
        if (onContextMenu) onContextMenu(item, e.clientX, e.clientY);
      }}
      onClick={(e) => {
        if (selectionMode || (e.target as HTMLElement).closest("button")) return;
        if (onClick) onClick(item);
      }}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={resetSwipe}
      onPointerLeave={resetSwipe}
    >
      <div className="aspect-video bg-[var(--color-muted)] relative overflow-hidden">
        {selectionMode && (
          <label className="absolute top-1 left-1 z-10 flex h-8 w-8 items-center justify-center rounded bg-black/60">
            <input type="checkbox" checked={selected} readOnly className="h-4 w-4" />
          </label>
        )}
        {thumb ? (
          <img
            src={thumb}
            alt={title}
            className="h-full w-full object-cover transition-transform duration-300 group-hover:scale-105"
            loading="lazy"
            onError={(e) => {
              (e.target as HTMLImageElement).style.display = "none";
            }}
          />
        ) : (
          <div className="flex h-full items-center justify-center text-[var(--color-muted-foreground)] text-xs">
            No preview
          </div>
        )}

        {showOverlay && (
          <div className="absolute top-1 right-1 z-10 flex gap-1 opacity-0 transition-opacity group-hover:opacity-100 md:opacity-0">
            {onEdit && (
              <Button
                size="sm"
                variant="secondary"
                className="h-8 w-8 p-0 bg-black/65 text-white hover:bg-black/80"
                onClick={(e) => {
                  e.stopPropagation();
                  onEdit(item as Scene);
                }}
                aria-label="Edit"
              >
                <Pencil className="h-3.5 w-3.5" />
              </Button>
            )}
            {onContextMenu && (
              <Button
                size="sm"
                variant="secondary"
                className="h-8 w-8 p-0 bg-black/65 text-white hover:bg-black/80"
                onClick={(e) => {
                  e.stopPropagation();
                  const rect = (e.target as HTMLElement).getBoundingClientRect();
                  onContextMenu(item, rect.left, rect.bottom + 4);
                }}
                aria-label="More actions"
              >
                <MoreVertical className="h-3.5 w-3.5" />
              </Button>
            )}
          </div>
        )}

        {duration != null && duration > 0 && (
          <span className="absolute bottom-2 right-2 rounded bg-black/70 px-1.5 py-0.5 text-xs flex items-center gap-1">
            <Clock className="h-3 w-3" />
            {formatDuration(duration)}
            {fileSize != null && fileSize > 0 && (
              <span className="text-[var(--color-muted-foreground)]">
                · {formatFileSize(fileSize)}
              </span>
            )}
          </span>
        )}
        {resolution && (
          <span className="absolute bottom-2 left-2 rounded bg-black/70 px-1.5 py-0.5 text-[10px] tabular-nums">
            {resolution}
          </span>
        )}
        {showWatched && !selectionMode && (
          <span className="absolute top-2 left-2 flex items-center gap-1 rounded bg-green-600/90 px-1.5 py-0.5 text-[10px] font-medium text-white">
            <Check className="h-3 w-3" />
            Watched
          </span>
        )}
        {watchFraction != null && watchFraction > 0 && (
          <div className="absolute inset-x-0 bottom-0 h-1 bg-black/60">
            <div
              className="h-full bg-red-500"
              style={{ width: `${Math.round(watchFraction * 100)}%` }}
            />
          </div>
        )}

        {onWatch && showOverlay && (
          <div className="absolute inset-0 flex items-center justify-center opacity-0 transition-opacity group-hover:opacity-100 bg-black/30">
            <Button
              size="sm"
              className="rounded-full h-12 w-12 p-0 bg-[var(--color-primary)] hover:bg-[var(--color-primary)]/80"
              onClick={(e) => {
                e.stopPropagation();
                onWatch(item as MediaItem);
              }}
              aria-label="Watch"
            >
              <Play className="h-5 w-5" />
            </Button>
          </div>
        )}

        {swipeX !== 0 && (
          <div
            className={`absolute inset-0 flex items-center justify-center transition-opacity duration-150 ${
              Math.abs(swipeX) > 40 ? "opacity-100" : "opacity-50"
            }`}
          >
            <div
              className={`rounded-full p-3 ${
                swipeX < 0 ? "bg-[var(--color-primary)]/90" : "bg-green-500/90"
              }`}
            >
              {swipeX < 0 ? (
                <Download className="h-5 w-5 text-white" />
              ) : (
                <Play className="h-5 w-5 text-white" />
              )}
            </div>
          </div>
        )}
      </div>

      <CardContent className="p-2.5 space-y-1.5">
        <p className="line-clamp-2 text-xs font-medium leading-tight">{title}</p>

        {(channel || performers.length > 0) && (
          <p className="text-[10px] text-[var(--color-muted-foreground)] truncate">
            {channel || performers.join(", ")}
          </p>
        )}

        {tags.length > 0 && (
          <div className="flex flex-wrap gap-1">
            {tags.slice(0, 3).map((tag) => (
              <span
                key={tag}
                className="rounded bg-[var(--color-secondary)] px-1.5 py-0.5 text-[10px]"
              >
                {tag}
              </span>
            ))}
          </div>
        )}

        {hasActions && (
          <div className="flex gap-1.5 pt-1">
            {onInfo && (
              <Button
                size="sm"
                variant="outline"
                className="flex-1 px-1.5 sm:px-3 h-8 text-xs"
                onClick={(e) => {
                  e.stopPropagation();
                  onInfo(item as MediaItem);
                }}
                aria-label="Info"
              >
                <Info className="h-3.5 w-3.5" />
                <span className="hidden sm:inline ml-1">Info</span>
              </Button>
            )}
            {onDownload && (
              <Button
                size="sm"
                variant={onWatch ? "outline" : "default"}
                className="flex-1 px-1.5 sm:px-3 h-8 text-xs"
                onClick={(e) => {
                  e.stopPropagation();
                  onDownload(item as MediaItem);
                }}
                aria-label="Download"
              >
                <Download className="h-3.5 w-3.5" />
                <span className="hidden sm:inline ml-1">Download</span>
              </Button>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function formatDuration(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = Math.round(seconds % 60);
  if (m >= 60) {
    const h = Math.floor(m / 60);
    const rm = m % 60;
    return `${h}:${rm.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  }
  return `${m}:${s.toString().padStart(2, "0")}`;
}

function formatFileSize(bytes: number): string {
  if (bytes >= 1073741824) {
    return `${(bytes / 1073741824).toFixed(1)} GB`;
  }
  if (bytes >= 1048576) {
    return `${(bytes / 1048576).toFixed(0)} MB`;
  }
  return `${(bytes / 1024).toFixed(0)} KB`;
}

export { formatDuration, formatFileSize };
