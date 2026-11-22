import { Link, createFileRoute, useNavigate } from "@tanstack/react-router";
import { useEffect, useMemo, useState } from "react";
import { api } from "@/lib/api/client";
import { Button } from "@/components/ui/button";
import type { Tag } from "@/lib/types";
import { ArrowUpRight, Check, Filter } from "lucide-react";

export const Route = createFileRoute("/library/tags/")({
  component: TagsPage,
});

type TagRow = Tag & { depth: number };

function buildTagTree(tags: Tag[]): TagRow[] {
  const byParent = new Map<string | undefined, Tag[]>();
  for (const tag of tags) {
    const key = tag.parent_id || undefined;
    const list = byParent.get(key) ?? [];
    list.push(tag);
    byParent.set(key, list);
  }
  for (const list of byParent.values()) {
    list.sort((a, b) => a.name.localeCompare(b.name));
  }

  const rows: TagRow[] = [];
  const visit = (parentId: string | undefined, depth: number, stack: Set<string>) => {
    const children = byParent.get(parentId) ?? [];
    for (const child of children) {
      if (stack.has(child.id)) continue;
      rows.push({ ...child, depth });
      const next = new Set(stack);
      next.add(child.id);
      visit(child.id, depth + 1, next);
    }
  };
  visit(undefined, 0, new Set());

  // Orphans whose parent is missing from the list still show at the root.
  const listed = new Set(rows.map((r) => r.id));
  for (const tag of tags) {
    if (!listed.has(tag.id)) {
      rows.push({ ...tag, depth: 0 });
    }
  }
  return rows;
}

function TagsPage() {
  const navigate = useNavigate();
  const [tags, setTags] = useState<Tag[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());

  useEffect(() => {
    void api.listTags().then(setTags).catch(console.error);
  }, []);

  const rows = useMemo(() => buildTagTree(tags), [tags]);

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
      <div className="flex flex-col gap-1.5">
        {rows.map((tag) => {
          const checked = selected.has(tag.name);
          return (
            <div
              key={tag.id}
              className={`inline-flex max-w-full items-center gap-1 rounded-full border pr-1 pl-3 py-1 text-sm transition-colors ${
                checked
                  ? "border-[var(--color-primary)] bg-[var(--color-primary)]/10"
                  : "border-[var(--color-border)] bg-[var(--color-card)] hover:border-[var(--color-primary)]"
              }`}
              style={{ marginLeft: `${tag.depth * 16}px` }}
            >
              <button
                type="button"
                onClick={() => toggle(tag.name)}
                className="inline-flex min-w-0 items-center gap-1.5"
                aria-pressed={checked}
              >
                {checked ? (
                  <Check className="h-3 w-3 shrink-0 text-[var(--color-primary)]" />
                ) : (
                  <span className="h-3 w-3 shrink-0 rounded-sm border border-[var(--color-border)]" />
                )}
                <span className="truncate">{tag.name}</span>
                <span className="ml-0.5 text-[var(--color-muted-foreground)]">
                  {tag.scene_count}
                </span>
              </button>
              <Link
                to="/library/tags/$tagId"
                params={{ tagId: tag.id }}
                className="flex min-h-9 min-w-9 items-center justify-center rounded-full text-[var(--color-muted-foreground)] hover:bg-[var(--color-accent)] hover:text-[var(--color-foreground)]"
                title={`Open ${tag.name}`}
                aria-label={`Open ${tag.name}`}
              >
                <ArrowUpRight className="h-3.5 w-3.5" />
              </Link>
            </div>
          );
        })}
      </div>
      {tags.length === 0 && (
        <p className="text-sm text-[var(--color-muted-foreground)]">No tags yet.</p>
      )}
    </div>
  );
}
