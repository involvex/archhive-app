import { useCallback, useEffect, useMemo, useState } from "react";
import { api } from "@/lib/api/client";
import { Button } from "@/components/ui/button";
import { HlsVideoPlayer, STREAM_URL_EXPIRED_ERROR } from "@/components/HlsVideoPlayer";
import { AlertCircle, ChevronLeft, ChevronRight, Loader2, X } from "lucide-react";
import type { MediaItem } from "@/lib/types";

interface UrlPlayerDialogProps {
  item: MediaItem | null;
  /** Browse playlist — enables Previous / Next when more than one item. */
  playlist?: MediaItem[];
  open: boolean;
  onClose: () => void;
  onSelectItem?: (item: MediaItem) => void;
}

export function UrlPlayerDialog({
  item,
  playlist,
  open,
  onClose,
  onSelectItem,
}: UrlPlayerDialogProps) {
  const [streamUrl, setStreamUrl] = useState<string | null>(null);
  const [embedUrl, setEmbedUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [queued, setQueued] = useState(false);

  const index = useMemo(() => {
    if (!item || !playlist?.length) return -1;
    return playlist.findIndex((p) => p.id === item.id || p.url === item.url);
  }, [item, playlist]);

  const prevItem = index > 0 ? playlist![index - 1] : null;
  const nextItem =
    index >= 0 && playlist && index < playlist.length - 1 ? playlist[index + 1] : null;

  // Resolve a fresh stream URL. Signed CDN URLs expire quickly, so playback
  // failures are retried through here rather than reusing a stale URL.
  const resolveStream = useCallback(async () => {
    if (!item) return;
    setLoading(true);
    setError(null);
    setStreamUrl(null);
    setEmbedUrl(null);
    try {
      // Live rooms: same path as /live — prefer direct HLS, fall back to embed.
      if (item.is_live) {
        const live = await api.resolveLivestream(item.url);
        if (live.embed_url?.trim()) {
          setEmbedUrl(live.embed_url.trim());
        }
        if (live.stream_url?.trim()) {
          setStreamUrl(live.stream_url.trim());
          return;
        }
        // Stream failed but embed may still work (Chaturbate iframe).
        if (live.embed_url?.trim()) {
          return;
        }
        setStreamUrl(await api.resolveStreamUrl(item.url));
        return;
      }
      // Prefer an already-resolved HLS URL from listings when not live.
      if (item.stream_url?.trim()) {
        setStreamUrl(item.stream_url.trim());
        return;
      }
      setStreamUrl(await api.resolveStreamUrl(item.url));
    } catch (e: unknown) {
      if (typeof e === "string" && e.trim()) {
        setError(e);
      } else if (e instanceof Error && e.message) {
        setError(e.message);
      } else {
        setError("Failed to resolve stream URL");
      }
    } finally {
      setLoading(false);
    }
  }, [item]);

  useEffect(() => {
    if (!open || !item) return;
    // eslint-disable-next-line react-hooks/set-state-in-effect -- fetch-on-open pattern
    setQueued(false);
    void resolveStream();
  }, [open, item, resolveStream]);

  useEffect(() => {
    if (!open) return;
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
        return;
      }
      if (e.key === "ArrowRight" && nextItem && onSelectItem) {
        e.preventDefault();
        onSelectItem(nextItem);
      }
      if (e.key === "ArrowLeft" && prevItem && onSelectItem) {
        e.preventDefault();
        onSelectItem(prevItem);
      }
    }
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [open, onClose, nextItem, prevItem, onSelectItem]);

  if (!open || !item) return null;

  return (
    <div
      role="presentation"
      className="fixed inset-0 z-[100] flex cursor-default items-stretch justify-center bg-black/80 p-0 sm:items-center sm:p-4"
      onClick={onClose}
      onKeyDown={(e) => {
        if (e.key === "Escape") onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={item.title}
        className="flex h-full max-h-[100dvh] w-full max-w-4xl cursor-default flex-col overflow-hidden border border-[var(--color-border)] bg-[var(--color-card)] shadow-xl sm:max-h-[92dvh] sm:rounded-lg"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(e) => e.stopPropagation()}
      >
        <div className="overflow-y-auto overscroll-contain p-4">
          <div className="mb-3 flex items-center justify-between gap-2">
            <h3 className="text-lg font-semibold leading-snug line-clamp-2">{item.title}</h3>
            <button
              type="button"
              onClick={onClose}
              className="shrink-0 rounded p-2 hover:bg-[var(--color-muted)]"
              aria-label="Close"
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          {loading && (
            <div className="flex aspect-video items-center justify-center rounded-md bg-[var(--color-muted)]">
              <Loader2 className="h-8 w-8 animate-spin text-[var(--color-muted-foreground)]" />
            </div>
          )}

          {error && !embedUrl && (
            <div className="flex aspect-video flex-col items-center justify-center gap-2 rounded-md bg-[var(--color-muted)]">
              <AlertCircle className="h-8 w-8 text-red-400" />
              <p className="px-4 text-center text-sm text-[var(--color-muted-foreground)]">
                {error}
              </p>
              {/403|forbidden|blocked/i.test(error) && (
                <p className="px-4 text-center text-xs text-[var(--color-muted-foreground)]">
                  This site is blocking the embedded player. Try downloading instead (it sends
                  Referer/cookie headers), import cookies in Settings → Cookies, or use Remote LAN
                  mode.
                </p>
              )}
            </div>
          )}

          {streamUrl && !error && (
            <HlsVideoPlayer
              src={streamUrl}
              autoPlay
              className="aspect-video w-full rounded-md bg-black"
              onError={(mediaError: MediaError | null) => {
                if (mediaError?.code === 4) {
                  setError(STREAM_URL_EXPIRED_ERROR);
                } else {
                  setError("Playback failed.");
                }
              }}
            />
          )}

          {!streamUrl && !loading && embedUrl && (
            <iframe
              src={embedUrl}
              className="aspect-video w-full rounded-md border-0 bg-black"
              title={`${item.title} live stream`}
              allow="autoplay; encrypted-media; fullscreen; picture-in-picture"
              allowFullScreen
            />
          )}

          {!streamUrl && !loading && embedUrl && (
            <p className="mt-2 text-xs text-[var(--color-muted-foreground)]">
              Playing embedded player (direct stream URL unavailable). Tap Retry for HLS if cookies
              are configured.
            </p>
          )}

          {(prevItem || nextItem) && onSelectItem && (
            <div className="mt-3 flex items-center justify-between gap-2">
              <Button
                variant="outline"
                disabled={!prevItem}
                onClick={() => prevItem && onSelectItem(prevItem)}
                className="min-h-10 gap-1"
              >
                <ChevronLeft className="h-4 w-4" />
                Previous
              </Button>
              {index >= 0 && playlist && (
                <span className="text-xs text-[var(--color-muted-foreground)]">
                  {index + 1} / {playlist.length}
                </span>
              )}
              <Button
                variant="outline"
                disabled={!nextItem}
                onClick={() => nextItem && onSelectItem(nextItem)}
                className="min-h-10 gap-1"
              >
                Next
                <ChevronRight className="h-4 w-4" />
              </Button>
            </div>
          )}

          <dl className="mt-4 space-y-2 text-sm">
            {item.performers.length > 0 && (
              <div>
                <dt className="text-xs text-[var(--color-muted-foreground)]">Performers</dt>
                <dd>{item.performers.join(", ")}</dd>
              </div>
            )}
            {item.channel && (
              <div>
                <dt className="text-xs text-[var(--color-muted-foreground)]">Channel</dt>
                <dd>{item.channel}</dd>
              </div>
            )}
            {item.tags.length > 0 && (
              <div>
                <dt className="text-xs text-[var(--color-muted-foreground)]">Tags</dt>
                <dd className="mt-1 flex flex-wrap gap-1">
                  {item.tags.map((tag) => (
                    <span
                      key={tag}
                      className="rounded bg-[var(--color-secondary)] px-1.5 py-0.5 text-xs"
                    >
                      {tag}
                    </span>
                  ))}
                </dd>
              </div>
            )}
            {item.description && (
              <div>
                <dt className="text-xs text-[var(--color-muted-foreground)]">Description</dt>
                <dd className="text-xs text-[var(--color-muted-foreground)] line-clamp-4">
                  {item.description}
                </dd>
              </div>
            )}
            <div>
              <dt className="text-xs text-[var(--color-muted-foreground)]">Source URL</dt>
              <dd className="break-all font-mono text-xs text-[var(--color-muted-foreground)]">
                {item.url}
              </dd>
            </div>
          </dl>

          <div className="mt-4 flex justify-end gap-2">
            {(error || (!streamUrl && embedUrl)) && (
              <Button
                variant="outline"
                onClick={() => void resolveStream()}
                disabled={loading}
                className="min-h-10 min-w-[5.5rem]"
              >
                {loading ? "Retrying…" : "Retry"}
              </Button>
            )}
            <Button
              variant="default"
              onClick={() => {
                if (!item) return;
                void api
                  .queueDownload(item.url, item.site_id, item.title)
                  .then(() => setQueued(true))
                  .catch((e: unknown) =>
                    setError(e instanceof Error ? e.message : "Failed to queue download"),
                  );
              }}
              className="min-h-10 min-w-[5.5rem]"
            >
              {queued ? "Queued ✓" : "Download"}
            </Button>
            <Button variant="outline" onClick={onClose} className="min-h-10 min-w-[5.5rem]">
              Close
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
