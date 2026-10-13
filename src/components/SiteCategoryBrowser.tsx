import { useCallback, useEffect, useMemo, useState } from "react";
import { Link, useNavigate } from "@tanstack/react-router";
import { Layers, RefreshCw, Search } from "lucide-react";
import { api } from "@/lib/api/client";
import type { FilterOption, SiteCategory } from "@/lib/types";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { ErrorState } from "@/components/ErrorState";
import { CombineBar } from "@/components/CombineBar";

/** Per-session cache so returning to a site hub doesn't refetch. */
const cache = new Map<string, SiteCategory[]>();

/** Most tags the picker lets you combine (sites without exact support run
 *  them as a word search, where more words just dilute relevance). */
const MAX_SEARCH_COMBINE = 4;

/** Cached category list for a site, or one group of a grouped index
 *  (shared with browse-page autocomplete). */
export async function loadSiteCategories(
  siteId: string,
  force = false,
  group?: string,
): Promise<SiteCategory[]> {
  const key = `${siteId}|${group ?? ""}`;
  if (!force) {
    const hit = cache.get(key);
    if (hit) return hit;
  }
  // `force` (the Refresh button) is the only thing that asks the site again;
  // otherwise the app's cached copy is used however old it is.
  const list = await api.listSiteCategories(siteId, group, force);
  cache.set(key, list);
  return list;
}

/** Group id holding tags that start with `text` (digits/other go to the
 *  site's "09" / "0" group). */
export function groupForText(text: string, groups: FilterOption[]): string | undefined {
  const first = text.trim().charAt(0).toLowerCase();
  if (!first || groups.length === 0) return undefined;
  if (groups.some((g) => g.id === first)) return first;
  // Digits / other characters: XVIDEOS calls that group "09", xHamster "0".
  return groups.find((g) => g.id === "09" || g.id === "0")?.id;
}

function formatCount(n?: number): string {
  if (n == null) return "";
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${Math.round(n / 1_000)}K`;
  return String(n);
}

interface SiteCategoryBrowserProps {
  siteId: string;
  siteName: string;
  /** Tags one URL can intersect exactly (`BrowseOptions.combine_tags`). */
  combineTags?: number;
  /** Grouped index sections (`BrowseOptions.category_groups`). */
  groups?: FilterOption[];
  /** Combined category listings can also exclude (`a,b,-c`, xPomelo). */
  canExclude?: boolean;
}

/** Category / tag index for a site, fetched from the backend adapter. */
export function SiteCategoryBrowser({
  siteId,
  siteName,
  combineTags = 0,
  groups = [],
  canExclude = false,
}: SiteCategoryBrowserProps) {
  const navigate = useNavigate();
  // "" = the site's main (popular) index; otherwise a group id such as "a".
  const [group, setGroup] = useState("");
  const [categories, setCategories] = useState<SiteCategory[] | null>(
    () => cache.get(`${siteId}|`) ?? null,
  );
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [filter, setFilter] = useState("");
  const [order, setOrder] = useState<"az" | "popular">("az");
  const [combining, setCombining] = useState(false);
  const [selected, setSelected] = useState<SiteCategory[]>([]);
  // Slugs of selected categories that are excluded rather than required.
  const [excluded, setExcluded] = useState<string[]>([]);

  const load = useCallback(
    async (force = false) => {
      setLoading(true);
      setError("");
      try {
        setCategories(await loadSiteCategories(siteId, force, group || undefined));
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e) || "Could not load categories");
      } finally {
        setLoading(false);
      }
    },
    [siteId, group],
  );

  useEffect(() => {
    const hit = cache.get(`${siteId}|${group}`);
    if (hit) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setCategories(hit);
      return;
    }
    setCategories(null);
    void load();
  }, [siteId, group, load]);

  const visible = useMemo(() => {
    const q = filter.trim().toLowerCase();
    const list = (categories ?? []).filter((c) => !q || c.name.toLowerCase().includes(q));
    return [...list].sort((a, b) =>
      order === "popular"
        ? (b.count ?? 0) - (a.count ?? 0) || a.name.localeCompare(b.name)
        : a.name.localeCompare(b.name, undefined, { sensitivity: "base", numeric: true }),
    );
  }, [categories, filter, order]);

  const kindLabel = categories?.every((c) => c.kind === "tag") ? "Tags" : "Categories";
  // XVIDEOS categories (/c/…) cannot be combined; only its tags can.
  const canCombine = !(groups.length > 0 && group === "categories");
  const exact = combineTags > 1;
  const maxPick = exact ? combineTags : MAX_SEARCH_COMBINE;
  const isSelected = (c: SiteCategory) => selected.some((s) => s.slug === c.slug);

  const isExcluded = (c: SiteCategory) => excluded.includes(c.slug);

  function toggle(c: SiteCategory) {
    // Required → excluded → cleared, where the site can exclude categories.
    if (canExclude && c.kind === "category" && isSelected(c)) {
      if (!isExcluded(c)) {
        setExcluded((prev) => [...prev, c.slug]);
        return;
      }
      setExcluded((prev) => prev.filter((s) => s !== c.slug));
    }
    setSelected((prev) =>
      prev.some((s) => s.slug === c.slug)
        ? prev.filter((s) => s.slug !== c.slug)
        : prev.length >= maxPick
          ? prev
          : [...prev, c],
    );
  }

  function browseSelected() {
    if (selected.length === 0) return;
    const required = selected.filter((s) => !excluded.includes(s.slug));
    void navigate({
      to: "/browse/$site/$kind/$slug",
      params: {
        site: siteId,
        kind: required[0]?.kind ?? "category",
        slug: selected.map((s) => (excluded.includes(s.slug) ? `-${s.slug}` : s.slug)).join(","),
      },
    });
  }

  const chipClass = (active: boolean) =>
    `inline-flex items-center gap-1.5 rounded-full px-3 py-1.5 text-sm transition ${
      active
        ? "bg-[var(--color-primary)] text-[var(--color-primary-foreground)]"
        : "bg-[var(--color-secondary)] hover:bg-[var(--color-muted)]"
    }`;

  // Named sections (main index, XVIDEOS "Categories") vs. A–Z letter pages.
  const sections = [
    {
      id: "",
      label: groups.some((g) => g.id === "categories") ? "Popular" : "Categories",
    },
    ...groups.filter((g) => g.label.length > 3),
  ];
  const letters = groups.filter((g) => g.label.length <= 3);
  const tabClass = (active: boolean) =>
    `rounded-lg text-sm font-medium transition ${
      active
        ? "bg-[var(--color-primary)] text-[var(--color-primary-foreground)]"
        : "bg-[var(--color-secondary)] hover:bg-[var(--color-muted)]"
    }`;

  return (
    <section className="space-y-3">
      <div className="flex items-center gap-2">
        <h3 className="min-w-0 flex-1 truncate text-base font-semibold">
          {kindLabel}
          {categories && (
            <span className="ml-2 text-xs font-normal text-[var(--color-muted-foreground)]">
              {categories.length.toLocaleString()}
            </span>
          )}
        </h3>
        {canCombine && (
          <Button
            size="sm"
            variant={combining ? "default" : "outline"}
            onClick={() => {
              setCombining((v) => !v);
              setSelected([]);
              setExcluded([]);
            }}
            title="Select several tags and browse videos matching them together"
          >
            <Layers className="h-4 w-4" />
            Combine
          </Button>
        )}
        <Button
          size="sm"
          variant="ghost"
          onClick={() => void load(true)}
          disabled={loading}
          title="Refresh"
          aria-label="Refresh"
        >
          <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
        </Button>
      </div>

      {groups.length > 0 && (
        <div className="space-y-2">
          <div className="flex gap-1.5">
            {sections.map((g) => (
              <button
                key={g.id || "main"}
                type="button"
                onClick={() => setGroup(g.id)}
                className={`flex-1 px-3 py-2 sm:flex-none ${tabClass(group === g.id)}`}
              >
                {g.label}
              </button>
            ))}
          </div>
          {letters.length > 0 && (
            <div className="grid grid-cols-9 gap-1 sm:flex sm:flex-wrap">
              {letters.map((g) => (
                <button
                  key={g.id}
                  type="button"
                  onClick={() => setGroup(g.id)}
                  aria-label={`Tags starting with ${g.label}`}
                  className={`h-9 sm:w-9 ${tabClass(group === g.id)}`}
                >
                  {g.label}
                </button>
              ))}
            </div>
          )}
        </div>
      )}

      <div className="flex items-center gap-2">
        <div className="relative min-w-0 flex-1">
          <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-[var(--color-muted-foreground)]" />
          <Input
            className="h-9 pl-8"
            placeholder={`Find ${kindLabel.toLowerCase()}…`}
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
          />
        </div>
        <div className="flex shrink-0 overflow-hidden rounded-md border border-[var(--color-border)] text-xs">
          {(
            [
              ["az", "A–Z"],
              ["popular", "Top"],
            ] as const
          ).map(([id, label]) => (
            <button
              key={id}
              type="button"
              onClick={() => setOrder(id)}
              title={id === "popular" ? "Most videos first" : "Alphabetical"}
              className={`h-9 px-3 ${order === id ? "bg-[var(--color-muted)] font-medium" : ""}`}
            >
              {label}
            </button>
          ))}
        </div>
      </div>

      {error && <ErrorState message={error} onRetry={() => void load(true)} />}
      {loading && !categories && (
        <p className="text-xs text-[var(--color-muted-foreground)]">Loading…</p>
      )}
      {categories && categories.length === 0 && !loading && !error && (
        <p className="text-xs text-[var(--color-muted-foreground)]">
          No categories returned. The site may require cookies — log in from Settings → Cookies.
        </p>
      )}
      <div className={`flex flex-wrap gap-1.5 ${combining ? "pb-24" : ""}`}>
        {visible.map((c) => {
          const inner = (
            <>
              {c.thumbnail && (
                <img
                  src={c.thumbnail}
                  alt=""
                  loading="lazy"
                  referrerPolicy="no-referrer"
                  className="h-5 w-5 rounded-full object-cover"
                />
              )}
              <span>{c.name}</span>
              {c.count != null && (
                <span className="text-[10px] opacity-70">{formatCount(c.count)}</span>
              )}
            </>
          );
          return combining && canCombine ? (
            <button
              key={`${c.kind}:${c.slug}`}
              type="button"
              className={
                isExcluded(c)
                  ? "inline-flex items-center gap-1.5 rounded-full bg-red-600 px-3 py-1.5 text-sm text-white line-through transition"
                  : chipClass(isSelected(c))
              }
              onClick={() => toggle(c)}
            >
              {inner}
            </button>
          ) : (
            <Link
              key={`${c.kind}:${c.slug}`}
              to="/browse/$site/$kind/$slug"
              params={{ site: siteId, kind: c.kind, slug: c.slug }}
              className={chipClass(false)}
            >
              {inner}
            </Link>
          );
        })}
      </div>

      {combining && canCombine && (
        <CombineBar
          selected={selected.map((c) => ({
            key: c.slug,
            label: isExcluded(c) ? `−${c.name}` : c.name,
          }))}
          hint={
            exact && canExclude
              ? `Tap up to ${maxPick}: once to require, twice to exclude, again to clear.`
              : exact
                ? `Tap up to ${maxPick} — ${siteName} shows videos tagged with all of them.`
                : `Tap up to ${maxPick} — ${siteName} can't match tags exactly, so they run as a word search.`
          }
          onRemove={(slug) => {
            setSelected((prev) => prev.filter((s) => s.slug !== slug));
            setExcluded((prev) => prev.filter((s) => s !== slug));
          }}
          onBrowse={browseSelected}
        />
      )}
    </section>
  );
}
