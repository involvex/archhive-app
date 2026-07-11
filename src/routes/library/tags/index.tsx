import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { Tag } from "@/lib/types";

export const Route = createFileRoute("/library/tags/")({
  component: TagsPage,
});

function TagsPage() {
  const [tags, setTags] = useState<Tag[]>([]);

  useEffect(() => {
    void api.listTags().then(setTags).catch(console.error);
  }, []);

  return (
    <div className="space-y-4">
      <h2 className="text-2xl font-bold">Tags</h2>
      <div className="flex flex-wrap gap-2">
        {tags.map((tag) => (
          <span
            key={tag.id}
            className="rounded-full border border-[var(--color-border)] bg-[var(--color-card)] px-3 py-1 text-sm"
          >
            {tag.name}
            <span className="ml-1.5 text-[var(--color-muted-foreground)]">{tag.scene_count}</span>
          </span>
        ))}
      </div>
      {tags.length === 0 && (
        <p className="text-sm text-[var(--color-muted-foreground)]">No tags yet.</p>
      )}
    </div>
  );
}
