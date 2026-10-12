import { useEffect, useMemo, useRef, useState } from "react";
import { ChevronDown, ChevronUp, Minus, Plus } from "lucide-react";
import type { BrowseFilters, SiteCategory } from "@/lib/types";
import { loadSiteCategories } from "@/components/SiteCategoryBrowser";
import { Input } from "@/components/ui/input";

function splitList(v?: string): string[] {
  return (v ?? "")
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
}

interface CategoryIncludeExcludeProps {
  siteId: string;
  filters: BrowseFilters;
  onFiltersChange: (f: BrowseFilters) => void;
  /** Most required categories here (0 = this listing can't require any). */
  maxInclude?: number;
  /** Most excluded categories. */
  maxExclude?: number;
}

type ChipState = "include" | "exclude" | "none";

/** Next state when a chip is tapped: neutral → include → exclude → neutral,
 *  skipping states that are unavailable or already full. */
export function nextChipState(
  state: ChipState,
  canInclude: boolean,
  canExclude: boolean,
): ChipState {
  if (state === "none") return canInclude ? "include" : canExclude ? "exclude" : "none";
  if (state === "include") return canExclude ? "exclude" : "none";
  return "none";
}

/**
 * "Must include" / "Exclude" category filters for sites whose listings take
 * them in the URL (BrowseOptions.include_exclude, e.g. Eporner
 * `?include=a,b&exclude=c`). Tapping a category cycles neutral → include →
 * exclude → neutral.
 */
export function CategoryIncludeExclude({
  siteId,
  filters,
  onFiltersChange,
  maxInclude = Infinity,
  maxExclude = Infinity,
}: CategoryIncludeExcludeProps) {
  const [open, setOpen] = useState(false);
  const [categories, setCategories] = useState<SiteCategory[]>([]);
  const [find, setFind] = useState("");
  // Local draft so several taps reload once: changes are committed after a
  // short pause (each committed change refetches the listing).
  const [draft, setDraft] = useState({ include: filters.include, exclude: filters.exclude });
  const commitTimer = useRef<number | undefined>(undefined);
  const include = splitList(draft.include);
  const exclude = splitList(draft.exclude);
  useEffect(() => {
    // Follow external resets (e.g. the filter bar's Reset button).
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setDraft({ include: filters.include, exclude: filters.exclude });
  }, [filters.include, filters.exclude]);
  useEffect(() => () => window.clearTimeout(commitTimer.current), []);

  useEffect(() => {
    if (!open || categories.length > 0) return;
    let cancelled = false;
    loadSiteCategories(siteId)
      .then((cats) => {
        if (!cancelled) setCategories(cats);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [open, siteId, categories.length]);

  const visible = useMemo(() => {
    const q = find.trim().toLowerCase();
    return categories
      .filter((c) => !q || c.name.toLowerCase().includes(q))
      .sort((a, b) => a.name.localeCompare(b.name));
  }, [categories, find]);

  function cycle(slug: string) {
    const state: ChipState = include.includes(slug)
      ? "include"
      : exclude.includes(slug)
        ? "exclude"
        : "none";
    const next = nextChipState(state, include.length < maxInclude, exclude.length < maxExclude);
    if (next === state) return;
    const inc = next === "include" ? [...include, slug] : include.filter((s) => s !== slug);
    const exc = next === "exclude" ? [...exclude, slug] : exclude.filter((s) => s !== slug);
    const draftNext = {
      include: inc.length ? inc.join(",") : undefined,
      exclude: exc.length ? exc.join(",") : undefined,
    };
    setDraft(draftNext);
    window.clearTimeout(commitTimer.current);
    commitTimer.current = window.setTimeout(
      () => onFiltersChange({ ...filters, ...draftNext }),
      900,
    );
  }

  const summary =
    include.length || exclude.length
      ? [
          include.length ? `${include.length} included` : "",
          exclude.length ? `${exclude.length} excluded` : "",
        ]
          .filter(Boolean)
          .join(", ")
      : "any";

  return (
    <div className="rounded-md border border-[var(--color-border)]">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center justify-between px-3 py-2 text-xs"
      >
        <span>
          <span className="font-medium">Categories:</span>{" "}
          <span className="text-[var(--color-muted-foreground)]">{summary}</span>
        </span>
        {open ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
      </button>
      {open && (
        <div className="space-y-2 border-t border-[var(--color-border)] p-2">
          <p className="text-[11px] text-[var(--color-muted-foreground)]">
            {maxInclude > 0
              ? "Tap once to require a category, twice to exclude it, again to clear."
              : "Tap to exclude a category, again to clear."}
            {maxInclude > 0 && maxInclude < Infinity && ` Up to ${maxInclude} required.`}
            {maxExclude < Infinity && ` Up to ${maxExclude} excluded.`}
          </p>
          <Input
            className="h-8 text-xs"
            placeholder="Find category…"
            value={find}
            onChange={(e) => setFind(e.target.value)}
          />
          <div className="flex max-h-56 flex-wrap gap-1.5 overflow-y-auto">
            {categories.length === 0 && (
              <span className="text-xs text-[var(--color-muted-foreground)]">Loading…</span>
            )}
            {visible.map((c) => {
              const state = include.includes(c.slug)
                ? "include"
                : exclude.includes(c.slug)
                  ? "exclude"
                  : "none";
              return (
                <button
                  key={c.slug}
                  type="button"
                  onClick={() => cycle(c.slug)}
                  aria-pressed={state !== "none"}
                  title={
                    state === "include"
                      ? "Required — tap to exclude"
                      : state === "exclude"
                        ? "Excluded — tap to clear"
                        : maxInclude > 0
                          ? "Tap to require"
                          : "Tap to exclude"
                  }
                  className={`inline-flex items-center gap-1 rounded-full px-2.5 py-1 text-xs transition ${
                    state === "include"
                      ? "bg-emerald-600 text-white"
                      : state === "exclude"
                        ? "bg-red-600 text-white line-through"
                        : "bg-[var(--color-secondary)] hover:bg-[var(--color-muted)]"
                  }`}
                >
                  {state === "include" && <Plus className="h-3 w-3" />}
                  {state === "exclude" && <Minus className="h-3 w-3" />}
                  {c.name}
                </button>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
