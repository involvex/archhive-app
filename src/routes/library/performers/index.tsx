import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { Performer } from "@/lib/types";
import { Input } from "@/components/ui/input";
import { Card, CardContent } from "@/components/ui/card";
import { Users } from "lucide-react";

export const Route = createFileRoute("/library/performers/")({
  component: PerformersPage,
});

function PerformersPage() {
  const [performers, setPerformers] = useState<Performer[]>([]);
  const [query, setQuery] = useState("");

  useEffect(() => {
    void api
      .listPerformers(query || undefined)
      .then(setPerformers)
      .catch(console.error);
  }, [query]);

  return (
    <div className="space-y-4">
      <h2 className="text-2xl font-bold">Performers</h2>
      <Input
        placeholder="Search performers..."
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        className="max-w-md"
      />
      <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
        {performers.map((p) => (
          <Card key={p.id}>
            <CardContent className="flex items-center gap-3 p-3">
              <div className="flex h-10 w-10 items-center justify-center rounded-full bg-[var(--color-muted)]">
                <Users className="h-5 w-5" />
              </div>
              <div>
                <p className="font-medium">{p.name}</p>
                <p className="text-xs text-[var(--color-muted-foreground)]">
                  {p.scene_count} scenes
                </p>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
      {performers.length === 0 && (
        <p className="text-sm text-[var(--color-muted-foreground)]">No performers yet.</p>
      )}
    </div>
  );
}
