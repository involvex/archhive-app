import { useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { MediaItem } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { X, ChevronDown, ChevronUp } from "lucide-react";

interface BrowseItemDetailsDialogProps {
  item: MediaItem | null;
  open: boolean;
  onClose: () => void;
}

function ExpandableText({
  label,
  text,
  maxChars = 280,
}: {
  label: string;
  text: string;
  maxChars?: number;
}) {
  const [expanded, setExpanded] = useState(false);
  const needsExpand = text.length > maxChars;
  const shown = expanded || !needsExpand ? text : `${text.slice(0, maxChars)}…`;

  return (
    <div>
      <dt className="text-xs text-[var(--color-muted-foreground)]">{label}</dt>
      <dd className="mt-0.5 whitespace-pre-wrap text-sm">{shown}</dd>
      {needsExpand && (
        <button
          type="button"
          className="mt-1 inline-flex items-center gap-1 text-xs text-[var(--color-primary)]"
          onClick={() => setExpanded((v) => !v)}
        >
          {expanded ? (
            <>
              Show less <ChevronUp className="h-3 w-3" />
            </>
          ) : (
            <>
              Show more <ChevronDown className="h-3 w-3" />
            </>
          )}
        </button>
      )}
    </div>
  );
}

function ExpandableList({ label, items }: { label: string; items: string[] }) {
  const [expanded, setExpanded] = useState(false);
  if (items.length === 0) return null;
  const shown = expanded ? items : items.slice(0, 8);
  return (
    <div>
      <dt className="text-xs text-[var(--color-muted-foreground)]">{label}</dt>
      <dd className="mt-1 flex flex-wrap gap-1">
        {shown.map((t) => (
          <span
            key={t}
            className="rounded bg-[var(--color-secondary)] px-1.5 py-0.5 text-xs"
            title="Click to copy"
            onClick={() => void navigator.clipboard.writeText(t).catch(() => undefined)}
            onKeyDown={() => undefined}
            role="button"
            tabIndex={0}
          >
            {t}
          </span>
        ))}
      </dd>
      {items.length > 8 && (
        <button
          type="button"
          className="mt-1 text-xs text-[var(--color-primary)]"
          onClick={() => setExpanded((v) => !v)}
        >
          {expanded ? "Show less" : `Show all (${items.length})`}
        </button>
      )}
    </div>
  );
}

function isThinMetadata(item: MediaItem): boolean {
  return (
    !item.description && !item.channel && item.performers.length === 0 && item.tags.length === 0
  );
}

function BrowseItemDetailsBody({ item, onClose }: { item: MediaItem; onClose: () => void }) {
  const [resolved, setResolved] = useState<MediaItem | null>(null);
  const [loading, setLoading] = useState(isThinMetadata(item));
  const [error, setError] = useState<string | null>(null);
  const [adding, setAdding] = useState<string | null>(null);
  const [addedNote, setAddedNote] = useState<string | null>(null);

  useEffect(() => {
    if (!isThinMetadata(item)) {
      return;
    }
    let cancelled = false;
    void api
      .resolveMediaDetails(item.url)
      .then((data) => {
        if (cancelled) return;
        setResolved({
          ...item,
          title: data.title || item.title,
          description: data.description ?? item.description,
          channel: data.channel ?? item.channel,
          thumbnail: data.thumbnail ?? item.thumbnail,
          duration: data.duration ?? item.duration,
          performers: data.performers.length > 0 ? data.performers : item.performers,
          tags: data.tags.length > 0 ? data.tags : item.tags,
        });
      })
      .catch((e) => {
        if (!cancelled) setError(e instanceof Error ? e.message : "Failed to resolve details");
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [item]);

  const data = resolved ?? item;

  async function addPerformer(name: string) {
    setAdding(name);
    setAddedNote(null);
    try {
      await api.ensurePerformer(name);
      setAddedNote(`Added “${name}” to library performers`);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to add performer");
    } finally {
      setAdding(null);
    }
  }

  const performerNames =
    data.performers.length > 0 ? data.performers : data.channel ? [data.channel] : [];

  return (
    <>
      <div className="mb-3 flex items-start justify-between gap-2">
        <h3 className="text-lg font-semibold leading-snug">{data.title}</h3>
        <button
          type="button"
          onClick={onClose}
          className="shrink-0 rounded p-1 hover:bg-[var(--color-muted)]"
        >
          <X className="h-4 w-4" />
        </button>
      </div>

      {data.thumbnail && (
        <div className="mb-3 aspect-video overflow-hidden rounded-md bg-[var(--color-muted)]">
          <img src={data.thumbnail} alt="" className="h-full w-full object-cover" />
        </div>
      )}

      {loading && (
        <p className="mb-2 text-sm text-[var(--color-muted-foreground)]">Loading details…</p>
      )}
      {error && <p className="mb-2 text-sm text-red-500">{error}</p>}
      {addedNote && <p className="mb-2 text-sm text-green-500">{addedNote}</p>}

      <dl className="space-y-3">
        {data.channel && (
          <div>
            <dt className="text-xs text-[var(--color-muted-foreground)]">Channel</dt>
            <dd className="text-sm">{data.channel}</dd>
          </div>
        )}
        {data.description && <ExpandableText label="Description" text={data.description} />}
        {performerNames.length > 0 && (
          <div>
            <dt className="text-xs text-[var(--color-muted-foreground)]">Performers</dt>
            <dd className="mt-1 space-y-1">
              {performerNames.map((name) => (
                <div key={name} className="flex items-center justify-between gap-2 text-sm">
                  <span>{name}</span>
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={adding === name}
                    onClick={() => void addPerformer(name)}
                  >
                    {adding === name ? "Adding…" : "Add to library"}
                  </Button>
                </div>
              ))}
            </dd>
          </div>
        )}
        <ExpandableList label="Tags" items={data.tags} />
        <div>
          <dt className="text-xs text-[var(--color-muted-foreground)]">URL</dt>
          <dd className="break-all text-xs text-[var(--color-muted-foreground)]">{data.url}</dd>
        </div>
      </dl>

      <div className="mt-4 flex justify-end">
        <Button variant="outline" onClick={onClose}>
          Close
        </Button>
      </div>
    </>
  );
}

export function BrowseItemDetailsDialog({ item, open, onClose }: BrowseItemDetailsDialogProps) {
  if (!open || !item) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
      <div
        role="dialog"
        aria-modal="true"
        className="max-h-[90vh] w-full max-w-lg overflow-y-auto rounded-lg border border-[var(--color-border)] bg-[var(--color-card)] p-4 shadow-xl"
      >
        <BrowseItemDetailsBody key={item.id} item={item} onClose={onClose} />
      </div>
    </div>
  );
}
