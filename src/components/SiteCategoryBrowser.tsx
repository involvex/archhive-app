import { useCallback, useEffect, useMemo, useState } from "react";
import { Link, useNavigate } from "@tanstack/react-router";
import { Layers, RefreshCw, Search, X } from "lucide-react";
import { api } from "@/lib/api/client";
import type { FilterOption, SiteCategory } from "@/lib/types";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { ErrorState } from "@/components/ErrorState";

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
  const list = await api.listSiteCategories(siteId, group);
  cache.set(key, list);
  return list;
}

/** Group id holding tags that start with `text` ("09" for digits/other). */
export function groupForText(text: string, groups: FilterOption[]): string | undefined {
  const first = text.trim().charAt(0).toLowerCase();
  if (!first || groups.length === 0) return undefined;
  if (groups.some((g) => g.id === first)) return first;
  return groups.find((g) => g.id === "09")?.id;
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
}

/** Category / tag index for a site, fetched from the backend adapter. */
export function SiteCategoryBrowser({
  siteId,
  siteName,
  combineTags = 0,
  groups = [],
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
  const exact = combineTags > 1;
  const maxPick = exact ? combineTags : MAX_SEARCH_COMBINE;
  const isSelected = (c: SiteCategory) => selected.some((s) => s.slug === c.slug);

  function toggle(c: SiteCategory) {
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
    void navigate({
      to: "/browse/$site/$kind/$slug",
      params: {
        site: siteId,
        kind: selected[0].kind,
        slug: selected.map((s) => s.slug).join(","),
      },
    });
  }

  const chipClass = (active: boolean) =>
    `inline-flex items-center gap-1.5 rounded-full px-3 py-1 text-xs transition ${
      active
        ? "bg-[var(--color-primary)] text-[var(--color-primary-foreground)]"
        : "bg-[var(--color-secondary)] hover:bg-[var(--color-muted)]"
    }`;

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between gap-2 space-y-0">
        <CardTitle className="text-base">
          {siteName} {kindLabel}
          {categories && (
            <span className="ml-2 text-xs font-normal text-[var(--color-muted-foreground)]">
              {categories.length}
            </span>
          )}
        </CardTitle>
        <Button
          size="sm"
          variant="ghost"
          onClick={() => void load(true)}
          disabled={loading}
          title="Refresh"
        >
          <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
        </Button>
      </CardHeader>
      <CardContent className="space-y-3">
        {groups.length > 0 && (
          <div className="flex gap-1 overflow-x-auto pb-1 scrollbar-hide">
            {[{ id: "", label: "Popular" }, ...groups].map((g) => (
              <button
                key={g.id || "popular"}
                type="button"
                onClick={() => setGroup(g.id)}
                className={`shrink-0 rounded-md px-2.5 py-1 text-xs font-medium ${
                  group === g.id
                    ? "bg-[var(--color-primary)] text-[var(--color-primary-foreground)]"
                    : "bg-[var(--color-secondary)] hover:bg-[var(--color-muted)]"
                }`}
              >
                {g.label}
              </button>
            ))}
          </div>
        )}
        <div className="flex flex-wrap items-center gap-2">
          <div className="relative min-w-40 flex-1 sm:max-w-sm">
            <Search className="absolute left-2 top-2.5 h-4 w-4 text-[var(--color-muted-foreground)]" />
            <Input
              className="pl-8"
              placeholder={`Find ${kindLabel.toLowerCase()}…`}
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
            />
          </div>
          <div className="flex overflow-hidden rounded-md border border-[var(--color-border)] text-xs">
            {(
              [
                ["az", "A–Z"],
                ["popular", "Most videos"],
              ] as const
            ).map(([id, label]) => (
              <button
                key={id}
                type="button"
                onClick={() => setOrder(id)}
                className={`px-2.5 py-1.5 ${order === id ? "bg-[var(--color-muted)] font-medium" : ""}`}
              >
                {label}
              </button>
            ))}
          </div>
          <Button
            size="sm"
            variant={combining ? "default" : "outline"}
            onClick={() => {
              setCombining((v) => !v);
              setSelected([]);
            }}
            title="Select several tags and browse videos matching them together"
          >
            <Layers className="h-4 w-4" />
            Combine
          </Button>
        </div>

        {combining && (
          <div className="space-y-2 rounded-md border border-[var(--color-border)] p-2">
            <p className="text-xs text-[var(--color-muted-foreground)]">
              {exact
                ? `Pick up to ${maxPick} — ${siteName} shows videos tagged with all of them.`
                : `Pick up to ${maxPick} — ${siteName} can't match tags exactly, so they run as a word search.`}
            </p>
            <div className="flex flex-wrap items-center gap-1.5">
              {selected.length === 0 && (
                <span className="text-xs text-[var(--color-muted-foreground)]">
                  Tap {kindLabel.toLowerCase()} below to add them.
                </span>
              )}
              {selected.map((c) => (
                <button
                  key={c.slug}
                  type="button"
                  className={chipClass(true)}
                  onClick={() => toggle(c)}
                >
                  {c.name}
                  <X className="h-3 w-3" />
                </button>
              ))}
              <Button
                size="sm"
                className="ml-auto"
                disabled={selected.length === 0}
                onClick={browseSelected}
              >
                Browse {selected.length > 1 ? `${selected.length} together` : ""}
              </Button>
            </div>
          </div>
        )}

        {error && <ErrorState message={error} onRetry={() => void load(true)} />}
        {loading && !categories && (
          <p className="text-xs text-[var(--color-muted-foreground)]">Loading…</p>
        )}
        {categories && categories.length === 0 && !loading && !error && (
          <p className="text-xs text-[var(--color-muted-foreground)]">
            No categories returned. The site may require cookies — log in from Settings → Cookies.
          </p>
        )}
        <div className="flex max-h-[60vh] flex-wrap gap-1.5 overflow-y-auto">
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
            return combining ? (
              <button
                key={`${c.kind}:${c.slug}`}
                type="button"
                className={chipClass(isSelected(c))}
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
      </CardContent>
    </Card>
  );
}
