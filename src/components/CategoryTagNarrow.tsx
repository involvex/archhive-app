import { useEffect, useMemo, useState } from "react";
import { Tag, X } from "lucide-react";
import type { BrowseFilters, FilterOption, SiteCategory } from "@/lib/types";
import { groupForText, loadSiteCategories } from "@/components/SiteCategoryBrowser";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";

interface CategoryTagNarrowProps {
  siteId: string;
  /** The site's tag index sections (`BrowseOptions.category_groups`). */
  groups: FilterOption[];
  filters: BrowseFilters;
  onFiltersChange: (f: BrowseFilters) => void;
}

/**
 * Narrow a category listing to videos that also have one tag — a strict
 * category ∩ tag match on sites with `BrowseOptions.category_tag_filter`
 * (ThisVid: /categories/<c>/?tag=<t>). Only one tag is supported there.
 */
export function CategoryTagNarrow({
  siteId,
  groups,
  filters,
  onFiltersChange,
}: CategoryTagNarrowProps) {
  const [text, setText] = useState(filters.tag ?? "");
  const [index, setIndex] = useState<SiteCategory[]>([]);
  const typed = text.trim().toLowerCase();
  const group = groupForText(typed, groups);

  useEffect(() => {
    // Follow external changes (filter bar Reset, navigation).
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setText(filters.tag ?? "");
  }, [filters.tag]);

  useEffect(() => {
    if (!group) return;
    let cancelled = false;
    loadSiteCategories(siteId, false, group)
      .then((cats) => {
        if (!cancelled) setIndex(cats);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [siteId, group]);

  const suggestions = useMemo(
    () =>
      typed
        ? index
            .filter((c) => c.slug.startsWith(typed) || c.name.toLowerCase().startsWith(typed))
            .slice(0, 50)
        : [],
    [index, typed],
  );

  function apply(value: string) {
    const tag = value
      .trim()
      .toLowerCase()
      .replace(/\s+/g, "-")
      .replace(/[^a-z0-9-]/g, "");
    onFiltersChange({ ...filters, tag: tag || undefined });
  }

  const listId = `narrow-tag-${siteId}`;
  return (
    <div className="flex items-center gap-2">
      <Tag className="h-4 w-4 shrink-0 text-[var(--color-muted-foreground)]" />
      <Input
        className="h-8 text-xs"
        list={listId}
        placeholder="Narrow by tag (videos in this category with this tag)"
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") apply(text);
        }}
      />
      <datalist id={listId}>
        {suggestions.map((c) => (
          <option key={c.slug} value={c.slug}>
            {c.name}
          </option>
        ))}
      </datalist>
      <Button size="sm" className="h-8" onClick={() => apply(text)} disabled={!typed}>
        Apply
      </Button>
      {filters.tag && (
        <Button
          size="sm"
          variant="ghost"
          className="h-8"
          title="Remove tag"
          onClick={() => onFiltersChange({ ...filters, tag: undefined })}
        >
          <X className="h-4 w-4" />
        </Button>
      )}
    </div>
  );
}
