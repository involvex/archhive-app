import { Link, createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { Studio } from "@/lib/types";
import { Input } from "@/components/ui/input";
import { EmptyState } from "@/components/EmptyState";
import { Clapperboard } from "lucide-react";

export const Route = createFileRoute("/library/studios/")({
  component: StudiosPage,
});

function StudiosPage() {
  const [studios, setStudios] = useState<Studio[]>([]);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    // eslint-disable-next-line react-hooks/set-state-in-effect -- async load keyed by query
    void (async () => {
      setLoading(true);
      try {
        const rows = await api.listStudios(query || undefined);
        if (!cancelled) setStudios(rows);
      } catch {
        if (!cancelled) setStudios([]);
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [query]);

  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-2xl font-bold">Studios</h2>
        <p className="text-sm text-[var(--color-muted-foreground)]">
          Grouped from studio name and channel metadata
        </p>
      </div>
      <Input
        placeholder="Search studios..."
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        className="max-w-md"
      />
      {loading ? (
        <p className="text-sm text-[var(--color-muted-foreground)]">Loading…</p>
      ) : studios.length === 0 ? (
        <EmptyState
          icon={<Clapperboard className="h-12 w-12" />}
          title="No studios yet"
          description="Studios appear when scenes have a studio name or channel."
        />
      ) : (
        <div className="space-y-2">
          {studios.map((studio) => (
            <Link
              key={studio.id}
              to="/library/studios/$studioId"
              params={{ studioId: studio.id }}
              className="flex items-center justify-between rounded-md border border-[var(--color-border)] p-3 hover:bg-[var(--color-muted)]/30"
            >
              <span className="font-medium">{studio.name}</span>
              <span className="text-xs text-[var(--color-muted-foreground)]">
                {studio.scene_count} scene{studio.scene_count !== 1 ? "s" : ""}
              </span>
            </Link>
          ))}
        </div>
      )}
    </div>
  );
}
