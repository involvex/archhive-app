import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { Scene } from "@/lib/types";
import { Input } from "@/components/ui/input";
import { Card, CardContent } from "@/components/ui/card";

export const Route = createFileRoute("/library/scenes/")({
  component: ScenesPage,
});

function ScenesPage() {
  const [scenes, setScenes] = useState<Scene[]>([]);
  const [query, setQuery] = useState("");

  useEffect(() => {
    void api
      .listScenes(query || undefined)
      .then(setScenes)
      .catch(console.error);
  }, [query]);

  return (
    <div className="space-y-4">
      <h2 className="text-2xl font-bold">Library — Scenes</h2>
      <Input
        placeholder="Search scenes..."
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        className="max-w-md"
      />
      <div className="grid grid-cols-2 gap-3 md:grid-cols-4 lg:grid-cols-5">
        {scenes.map((scene) => (
          <Card key={scene.id} className="overflow-hidden">
            <div className="aspect-video bg-[var(--color-muted)]">
              {scene.thumb && (
                <img src={scene.thumb} alt={scene.title} className="h-full w-full object-cover" />
              )}
            </div>
            <CardContent className="p-2 space-y-1">
              <p className="line-clamp-2 text-xs font-medium">{scene.title}</p>
              {scene.performers.length > 0 && (
                <p className="text-[10px] text-[var(--color-muted-foreground)] truncate">
                  {scene.performers.join(", ")}
                </p>
              )}
              <div className="flex flex-wrap gap-1">
                {scene.tags.slice(0, 3).map((tag) => (
                  <span
                    key={tag}
                    className="rounded bg-[var(--color-secondary)] px-1.5 py-0.5 text-[10px]"
                  >
                    {tag}
                  </span>
                ))}
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
      {scenes.length === 0 && (
        <p className="text-sm text-[var(--color-muted-foreground)]">No scenes in library.</p>
      )}
    </div>
  );
}
