import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import { Button } from "@/components/ui/button";
import type { Tag } from "@/lib/types";
import { Check, Filter } from "lucide-react";

export const Route = createFileRoute("/library/tags/")({
  component: TagsPage,
});

function TagsPage() {
  const navigate = useNavigate();
  const [tags, setTags] = useState<Tag[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());

  useEffect(() => {
    void api.listTags().then(setTags).catch(console.error);
  }, []);

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
      search: { tags: [...selected] },
    });
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold">Tags</h2>
        {selected.size > 0 && (
          <Button size="sm" onClick={applyFilter}>
            <Filter className="mr-1 h-3.5 w-3.5" />
            Filter scenes ({selected.size})
          </Button>
        )}
      </div>
      <div className="flex flex-wrap gap-2">
        {tags.map((tag) => {
          const checked = selected.has(tag.name);
          return (
            <button
              key={tag.id}
              type="button"
              onClick={() => toggle(tag.name)}
              className={`inline-flex items-center gap-1.5 rounded-full border px-3 py-1 text-sm transition-colors ${
                checked
                  ? "border-[var(--color-primary)] bg-[var(--color-primary)]/10"
                  : "border-[var(--color-border)] bg-[var(--color-card)] hover:border-[var(--color-primary)]"
              }`}
            >
              {checked ? (
                <Check className="h-3 w-3 text-[var(--color-primary)]" />
              ) : (
                <span className="h-3 w-3 rounded-sm border border-[var(--color-border)]" />
              )}
              {tag.name}
              <span className="ml-0.5 text-[var(--color-muted-foreground)]">{tag.scene_count}</span>
            </button>
          );
        })}
      </div>
      {tags.length === 0 && (
        <p className="text-sm text-[var(--color-muted-foreground)]">No tags yet.</p>
      )}
    </div>
  );
}
