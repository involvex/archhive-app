import { useCallback, useEffect, useMemo, useState } from "react";
import { api, isRoomOfflineError, safeMediaUrl } from "@/lib/api/client";
import { Button } from "@/components/ui/button";
import { HlsVideoPlayer, STREAM_URL_EXPIRED_ERROR } from "@/components/HlsVideoPlayer";
import {
  AlertCircle,
  Bookmark,
  BookmarkCheck,
  ChevronLeft,
  ChevronRight,
  Loader2,
  X,
} from "lucide-react";
import type { MediaItem } from "@/lib/types";
import { ExtLink } from "@/components/ExtLink";

interface UrlPlayerDialogProps {
  item: MediaItem | null;
  /** Browse playlist — enables Previous / Next when more than one item. */
  playlist?: MediaItem[];
  open: boolean;
  onClose: () => void;
  onSelectItem?: (item: MediaItem) => void;
  /** The site removed this video (the backend already dropped it from the
   * Online library); lets the page take it off its list too. */
  onRemoved?: (item: MediaItem) => void;
}

export function UrlPlayerDialog({
  item,
  playlist,
  open,
  onClose,
  onSelectItem,
  onRemoved,
}: UrlPlayerDialogProps) {
  const [streamUrl, setStreamUrl] = useState<string | null>(null);
  const [audioUrl, setAudioUrl] = useState<string | null>(null);
  const [embedUrl, setEmbedUrl] = useState<string | null>(null);
  const [streamError, setStreamError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [queued, setQueued] = useState(false);
  // Tags / performers / channel read from the video's own page when the
  // listing had no tags (null until fetched; `url` ties them to the item).
  const [details, setDetails] = useState<{ url: string; data: MediaItem } | null>(null);
  const [detailsLoading, setDetailsLoading] = useState(false);
  const [bookmarked, setBookmarked] = useState(false);
  const [bookmarking, setBookmarking] = useState(false);

  // The item with whatever its page added.
  const shown = useMemo((): MediaItem | null => {
    if (!item) return null;
    const d = details?.url === item.url ? details.data : null;
    if (!d) return item;
    return {
      ...item,
      performers: d.performers.length > 0 ? d.performers : item.performers,
      tags: d.tags.length > 0 ? d.tags : item.tags,
      channel: d.channel ?? item.channel,
      description: d.description ?? item.description,
      duration: item.duration ?? d.duration,
    };
  }, [item, details]);

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
    setAudioUrl(null);
    setEmbedUrl(null);
    setStreamError(null);
    try {
      // Live rooms: same path as /live — prefer direct HLS, fall back to embed.
      if (item.is_live) {
        const live = await api.resolveLivestream(item.url);
        if (live.embed_url?.trim()) {
          setEmbedUrl(live.embed_url.trim());
        }
        if (live.audio_url?.trim()) {
          setAudioUrl(live.audio_url.trim());
        }
        if (live.stream_error?.trim()) {
          setStreamError(live.stream_error.trim());
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
      // Listing payloads are scraped, so validate before feeding <video src>.
      const listingStream = safeMediaUrl(item.stream_url);
      if (listingStream) {
        setStreamUrl(listingStream);
        return;
      }
      setStreamUrl(await api.resolveStreamUrl(item.url));
    } catch (e: unknown) {
      const msg =
        typeof e === "string" && e.trim()
          ? e
          : e instanceof Error && e.message
            ? e.message
            : "Failed to resolve stream URL";
      setError(msg);
      if (msg.includes("video removed from the site")) onRemoved?.(item);
    } finally {
      setLoading(false);
    }
  }, [item, onRemoved]);

  useEffect(() => {
    if (!open || !item) return;
    // eslint-disable-next-line react-hooks/set-state-in-effect -- fetch-on-open pattern
    setQueued(false);
    void resolveStream();
  }, [open, item, resolveStream]);

  // Fetch tags from the video's page (one request) when the listing had
  // none; the backend also stores them on the Online library entry.
  useEffect(() => {
    if (!open || !item || item.is_live || item.tags.length > 0) return;
    let cancelled = false;
    // eslint-disable-next-line react-hooks/set-state-in-effect -- fetch-on-open pattern
    setDetailsLoading(true);
    api
      .resolveMediaDetails(item.url)
      .then((data) => {
        if (!cancelled) setDetails({ url: item.url, data });
      })
      .catch((e: unknown) => {
        const msg = e instanceof Error ? e.message : String(e ?? "");
        if (!cancelled && msg.includes("video removed from the site")) onRemoved?.(item);
      })
      .finally(() => {
        if (!cancelled) setDetailsLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [open, item, onRemoved]);

  useEffect(() => {
    if (!open || !item || item.is_live) return;
    let cancelled = false;
    api
      .isOnlineBookmarked(item.url)
      .then((yes) => {
        if (!cancelled) setBookmarked(yes);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
      setBookmarked(false);
    };
  }, [open, item]);

  const toggleBookmark = useCallback(async () => {
    if (!shown) return;
    setBookmarking(true);
    try {
      if (bookmarked) {
        await api.unbookmarkOnline(shown.url);
        setBookmarked(false);
      } else {
        // Saved with the tags fetched above, so it is searchable right away.
        await api.bookmarkOnline(shown);
        setBookmarked(true);
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBookmarking(false);
    }
  }, [shown, bookmarked]);

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

  if (!open || !item || !shown) return null;

  return (
    <div
      role="presentation"
      className="fixed inset-0 z-[100] flex cursor-default items-end justify-center bg-black/80 p-0 sm:items-center sm:p-4"
      onClick={onClose}
      onKeyDown={(e) => {
        if (e.key === "Escape") onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={item.title}
        className="flex max-h-[92dvh] w-full max-w-4xl cursor-default flex-col overflow-hidden rounded-t-xl border border-[var(--color-border)] bg-[var(--color-card)] shadow-xl sm:max-h-[92dvh] sm:rounded-lg"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(e) => e.stopPropagation()}
      >
        <div className="flex shrink-0 items-center justify-between gap-2 px-4 pt-safe-top pb-2">
          <h3 className="text-lg font-semibold leading-snug line-clamp-2">{item.title}</h3>
          <button
            type="button"
            onClick={onClose}
            className="flex min-h-11 min-w-11 shrink-0 items-center justify-center rounded hover:bg-[var(--color-muted)]"
            aria-label="Close"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="flex-1 min-h-0 overflow-y-auto overscroll-contain px-4 pb-4">
          {loading && (
            <div className="flex aspect-video max-h-[min(52dvh,26rem)] w-full items-center justify-center rounded-md bg-[var(--color-muted)]">
              <Loader2 className="h-8 w-8 animate-spin text-[var(--color-muted-foreground)]" />
            </div>
          )}

          {error && !embedUrl && (
            <div className="flex aspect-video max-h-[min(52dvh,26rem)] w-full flex-col items-center justify-center gap-2 rounded-md bg-[var(--color-muted)]">
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
              audioSrc={audioUrl ?? undefined}
              autoPlay
              className="aspect-video max-h-[min(52dvh,26rem)] w-full rounded-md bg-black object-contain"
              onError={(mediaError: MediaError | null) => {
                if (mediaError?.code === 4) {
                  setError(STREAM_URL_EXPIRED_ERROR);
                } else {
                  setError("Playback failed.");
                }
              }}
            />
          )}

          {!streamUrl && !loading && embedUrl && streamError && isRoomOfflineError(streamError) && (
            <div className="flex aspect-video max-h-[min(52dvh,26rem)] w-full items-center justify-center rounded-md bg-[var(--color-muted)]">
              <p className="px-4 text-center text-sm text-[var(--color-muted-foreground)]">
                This room looks offline or is in a private show right now — nothing to play.
              </p>
            </div>
          )}

          {!streamUrl &&
            !loading &&
            embedUrl &&
            (!streamError || !isRoomOfflineError(streamError)) && (
              <iframe
                src={embedUrl}
                className="aspect-video max-h-[min(52dvh,26rem)] w-full rounded-md border-0 bg-black object-contain"
                style={{ pointerEvents: "auto", touchAction: "manipulation" }}
                title={`${item.title} live stream`}
                allow="autoplay; encrypted-media; fullscreen; picture-in-picture"
                allowFullScreen
              />
            )}

          {!streamUrl &&
            !loading &&
            embedUrl &&
            (!streamError || !isRoomOfflineError(streamError)) && (
              <div className="mt-2 space-y-1 text-xs text-[var(--color-muted-foreground)]">
                <p>
                  Playing the site embed player. If its controls don&apos;t respond, open the room
                  URL below in your browser, or tap Retry for HLS if cookies are configured.
                </p>
                {streamError && (
                  <p className="font-mono break-all">Direct HLS failed: {streamError}</p>
                )}
              </div>
            )}

          <dl className="mt-4 space-y-2 text-sm">
            {shown.performers.length > 0 && (
              <div>
                <dt className="text-xs text-[var(--color-muted-foreground)]">Performers</dt>
                <dd>{shown.performers.join(", ")}</dd>
              </div>
            )}
            {shown.channel && (
              <div>
                <dt className="text-xs text-[var(--color-muted-foreground)]">Channel</dt>
                <dd>{shown.channel}</dd>
              </div>
            )}
            {detailsLoading && shown.tags.length === 0 && (
              <div className="flex items-center gap-1.5 text-xs text-[var(--color-muted-foreground)]">
                <Loader2 className="h-3 w-3 animate-spin" />
                Reading tags from the video page…
              </div>
            )}
            {shown.tags.length > 0 && (
              <div>
                <dt className="text-xs text-[var(--color-muted-foreground)]">Tags</dt>
                <dd className="mt-1 flex flex-wrap gap-1">
                  {shown.tags.map((tag) => (
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
            {shown.description && (
              <div>
                <dt className="text-xs text-[var(--color-muted-foreground)]">Description</dt>
                <dd className="text-xs text-[var(--color-muted-foreground)] line-clamp-4">
                  {shown.description}
                </dd>
              </div>
            )}
            <div>
              <dt className="text-xs text-[var(--color-muted-foreground)]">Source URL</dt>
              <dd className="break-all font-mono text-xs">
                <ExtLink href={item.url} title="Open in browser">
                  {item.url}
                </ExtLink>
              </dd>
            </div>
          </dl>
        </div>

        <div className="shrink-0 space-y-2 border-t border-[var(--color-border)] px-4 pt-2 pb-[max(0.75rem,env(safe-area-inset-bottom))]">
          {(prevItem || nextItem) && onSelectItem && (
            <div className="flex items-center justify-between gap-2">
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
          <div className="flex flex-wrap justify-end gap-2">
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
            {!item.is_live && (
              <Button
                variant={bookmarked ? "secondary" : "outline"}
                onClick={() => void toggleBookmark()}
                disabled={bookmarking}
                className="min-h-10 gap-1"
                title={
                  bookmarked
                    ? "Remove from Online library → Bookmarks"
                    : "Keep in Online library → Bookmarks"
                }
              >
                {bookmarked ? (
                  <BookmarkCheck className="h-4 w-4" />
                ) : (
                  <Bookmark className="h-4 w-4" />
                )}
                {bookmarked ? "Bookmarked" : "Bookmark"}
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
