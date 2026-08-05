import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { Performer } from "@/lib/types";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Check, Filter, Users } from "lucide-react";

export const Route = createFileRoute("/library/performers/")({
  component: PerformersPage,
});

function PerformersPage() {
  const navigate = useNavigate();
  const [performers, setPerformers] = useState<Performer[]>([]);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<Set<string>>(new Set());

  useEffect(() => {
    void api
      .listPerformers(query || undefined)
      .then(setPerformers)
      .catch(console.error);
  }, [query]);

  function toggle(name: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(name)) {
        next.delete(name);
      } else {
        next.add(name);
      }
      return next;
    });
  }

  function applyFilter() {
    if (selected.size === 0) return;
    navigate({
      to: "/library/scenes",
      search: { performers: [...selected] },
    });
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold">Performers</h2>
        {selected.size > 0 && (
          <Button size="sm" onClick={applyFilter}>
            <Filter className="mr-1 h-3.5 w-3.5" />
            Filter scenes ({selected.size})
          </Button>
        )}
      </div>
      <Input
        placeholder="Search performers..."
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        className="max-w-md"
      />
      <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
        {performers.map((p) => {
          const checked = selected.has(p.name);
          return (
            <Card
              key={p.id}
              className={`cursor-pointer transition-colors ${
                checked
                  ? "border-[var(--color-primary)] bg-[var(--color-primary)]/5"
                  : "hover:border-[var(--color-primary)]"
              }`}
              onClick={() => toggle(p.name)}
            >
              <CardContent className="flex items-center gap-3 p-3">
                <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-[var(--color-muted)]">
                  {checked ? (
                    <Check className="h-5 w-5 text-[var(--color-primary)]" />
                  ) : (
                    <Users className="h-5 w-5" />
                  )}
                </div>
                <div>
                  <p className="font-medium">{p.name}</p>
                  <p className="text-xs text-[var(--color-muted-foreground)]">
                    {p.scene_count} scenes
                  </p>
                </div>
              </CardContent>
            </Card>
          );
        })}
      </div>
      {performers.length === 0 && (
        <p className="text-sm text-[var(--color-muted-foreground)]">No performers yet.</p>
      )}
    </div>
  );
}
