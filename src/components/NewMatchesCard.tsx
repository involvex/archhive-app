import { useState } from "react";
import { api } from "@/lib/api/client";
import { MAX_NEW_MATCHES, type NewMatches } from "@/lib/hooks/useSavedSearches";
import { SceneCard } from "@/components/SceneCard";
import { Button } from "@/components/ui/button";
import { Download } from "lucide-react";

interface NewMatchesCardProps {
  matches: NewMatches;
  queueing: boolean;
  onQueueAll: () => void | Promise<void>;
  onDismiss: () => void | Promise<void>;
}

/**
 * Result card for a saved-search check. Shared by /browse and /feed.
 *
 * "Queue all" is destructive (starts N real downloads from scraped URLs), so
 * it routes through a confirm step showing the exact count. The hook already
 * filters to http(s) URLs via `queueableUrls`, so the count shown here is the
 * count that will actually be queued.
 */
export function NewMatchesCard({ matches, queueing, onQueueAll, onDismiss }: NewMatchesCardProps) {
  const [confirming, setConfirming] = useState(false);
  const count = Math.min(matches.items.length, MAX_NEW_MATCHES);
  const truncated = matches.total > matches.items.length;

  if (confirming) {
    return (
      <div className="rounded-md border border-[var(--color-primary)] bg-[var(--color-primary)]/10 p-4">
        <p className="text-sm">
          Queue {count} download{count === 1 ? "" : "s"} from{" "}
          <span className="font-medium">{matches.name}</span>?
          {truncated && (
            <span className="text-[var(--color-muted-foreground)]">
              {" "}
              ({matches.total} new in total)
            </span>
          )}
        </p>
        <div className="mt-3 flex gap-2">
          <Button
            size="sm"
            onClick={() => {
              setConfirming(false);
              void onQueueAll();
            }}
            disabled={queueing}
          >
            <Download className="h-3.5 w-3.5 mr-1.5" />
            {queueing ? "Queueing…" : `Queue ${count}`}
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setConfirming(false)}>
            Cancel
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="rounded-md border border-[var(--color-primary)] bg-[var(--color-primary)]/10 p-4">
      <p className="text-sm">
        {matches.total} new {matches.total === 1 ? "match" : "matches"} in{" "}
        <span className="font-medium">{matches.name}</span>
        {truncated && (
          <span className="text-[var(--color-muted-foreground)]">
            {" "}
            (showing first {matches.items.length})
          </span>
        )}
      </p>
      <div className="mt-3 flex gap-2">
        <Button size="sm" onClick={() => setConfirming(true)} disabled={queueing}>
          <Download className="h-3.5 w-3.5 mr-1.5" />
          Queue all
        </Button>
        <Button size="sm" variant="ghost" onClick={() => void onDismiss()}>
          Dismiss
        </Button>
      </div>
      <div className="mt-3 grid grid-cols-3 gap-3 sm:grid-cols-4 lg:grid-cols-5">
        {matches.items.map((item) => (
          <SceneCard
            key={item.id}
            item={item}
            onDownload={(i) => void api.queueDownload(i.url, item.site_id)}
          />
        ))}
      </div>
    </div>
  );
}
