import { useEffect, useRef, useState } from "react";
import { api } from "@/lib/api/client";
import { Button } from "@/components/ui/button";
import { AlertCircle, Loader2, X } from "lucide-react";
import type { MediaItem } from "@/lib/types";

interface UrlPlayerDialogProps {
  item: MediaItem | null;
  open: boolean;
  onClose: () => void;
}

export function UrlPlayerDialog({ item, open, onClose }: UrlPlayerDialogProps) {
  const [streamUrl, setStreamUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const videoRef = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    if (!open || !item) return;
    let cancelled = false;
    // eslint-disable-next-line react-hooks/set-state-in-effect -- standard fetch-in-effect pattern
    setLoading(true);
    setError(null);
    setStreamUrl(null);
    void api
      .resolveStreamUrl(item.url)
      .then((url) => {
        if (!cancelled) setStreamUrl(url);
      })
      .catch((e: unknown) => {
        if (cancelled) return;
        if (typeof e === "string" && e.trim()) {
          setError(e);
        } else if (e instanceof Error && e.message) {
          setError(e.message);
        } else {
          setError("Failed to resolve stream URL");
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [open, item]);

  useEffect(() => {
    if (!open) return;
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    }
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [open, onClose]);

  if (!open || !item) return null;

  return (
    <button
      type="button"
      aria-label="Close player"
      className="fixed inset-0 z-[100] flex cursor-default items-stretch justify-center bg-black/80 p-0 sm:items-center sm:p-4"
      onClick={onClose}
    >
      <div
        role="dialog"
        aria-modal="true"
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

          {error && (
            <div className="flex aspect-video flex-col items-center justify-center gap-2 rounded-md bg-[var(--color-muted)]">
              <AlertCircle className="h-8 w-8 text-red-400" />
              <p className="px-4 text-center text-sm text-[var(--color-muted-foreground)]">
                {error}
              </p>
            </div>
          )}

          {streamUrl && !error && (
            <video
              ref={videoRef}
              key={streamUrl}
              src={streamUrl}
              controls
              playsInline
              autoPlay
              className="aspect-video w-full rounded-md bg-black"
              onError={() => setError("Playback failed — the stream URL may have expired.")}
            >
              <track kind="captions" />
            </video>
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

          <div className="mt-4 flex justify-end">
            <Button variant="outline" onClick={onClose} className="min-h-10 min-w-[5.5rem]">
              Close
            </Button>
          </div>
        </div>
      </div>
    </button>
  );
}
