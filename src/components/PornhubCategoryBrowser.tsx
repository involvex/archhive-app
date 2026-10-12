import { useCallback, useEffect, useMemo, useState } from "react";
import { Link, useNavigate } from "@tanstack/react-router";
import { Layers, RefreshCw, X } from "lucide-react";
import { api } from "@/lib/api/client";
import {
  categoriesForOrientation,
  categoryBrowseSlug,
  mergeCategoryCatalog,
  PORNHUB_CATEGORIES,
  PORNHUB_LANGUAGES,
  PORNHUB_ORIENTATIONS,
  type BrowseOrientation,
  type PornhubCategory,
} from "@/lib/sites/pornhub-categories";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

/** The trans section's "All trans" entry (category 83), pinned on top. */
const isTransHub = (c: PornhubCategory) =>
  c.orientation === "transgender" && c.slug === "transgender";

/** Live catalog per orientation, kept for the session. */
const liveCache = new Map<BrowseOrientation, PornhubCategory[]>();

/**
 * PornHub category picker. Selecting a category opens the full browse page
 * (sort / filters / infinite scroll) with the orientation carried in the URL.
 */
export function PornhubCategoryBrowser() {
  const [orientation, setOrientation] = useState<BrowseOrientation>("straight");
  const [filter, setFilter] = useState("");
  const [liveCatalog, setLiveCatalog] = useState<PornhubCategory[] | null>(
    () => liveCache.get("straight") ?? null,
  );
  const [refreshing, setRefreshing] = useState(false);
  const [refreshStatus, setRefreshStatus] = useState("");
  const navigate = useNavigate();
  const [combining, setCombining] = useState(false);
  const [selected, setSelected] = useState<PornhubCategory[]>([]);
  // PornHub intersects exactly two straight or gay categories
  // (/[gay/]video/incategories/a/b); other orientations run as a word search.
  const exact = orientation === "straight" || orientation === "gay";
  const maxPick = exact ? 2 : 4;

  function toggle(cat: PornhubCategory) {
    setSelected((prev) =>
      prev.some((c) => c.slug === cat.slug)
        ? prev.filter((c) => c.slug !== cat.slug)
        : prev.length >= maxPick
          ? prev
          : [...prev, cat],
    );
  }

  const categories = useMemo(() => {
    const live = liveCatalog?.filter((c) => c.orientation === orientation);
    const list = live?.length ? live : categoriesForOrientation(orientation);
    const q = filter.trim().toLowerCase();
    if (!q) return list;
    return list.filter((c) => c.name.toLowerCase().includes(q));
  }, [orientation, filter, liveCatalog]);

  const refreshCategories = useCallback(
    async (force: boolean) => {
      const cached = liveCache.get(orientation);
      if (cached && !force) {
        setLiveCatalog(cached);
        return;
      }
      setRefreshing(true);
      setRefreshStatus("");
      try {
        const live = await api.listPornhubCategories(orientation);
        const staticForOrientation = PORNHUB_CATEGORIES.filter(
          (c) => c.orientation === orientation,
        );
        // Live entries win; static ones only fill gaps (e.g. offline).
        const merged = (
          live.length
            ? mergeCategoryCatalog([], live)
            : mergeCategoryCatalog(staticForOrientation, live)
        ).sort((a, b) => Number(isTransHub(b)) - Number(isTransHub(a)));
        liveCache.set(orientation, merged);
        setLiveCatalog(merged);
        if (force) setRefreshStatus(`Loaded ${merged.length} categories from PornHub.`);
      } catch (e) {
        setRefreshStatus(
          `${e instanceof Error ? e.message : String(e)} — showing the built-in list. Import PornHub cookies if this persists.`,
        );
      } finally {
        setRefreshing(false);
      }
    },
    [orientation],
  );

  useEffect(() => {
    // Load (or reuse) the live catalog whenever the orientation changes.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void refreshCategories(false);
  }, [refreshCategories]);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base">Categories</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="flex flex-wrap gap-2">
          {PORNHUB_ORIENTATIONS.map((o) => (
            <Button
              key={o.id}
              size="sm"
              variant={orientation === o.id ? "default" : "outline"}
              onClick={() => {
                setOrientation(o.id);
                setFilter("");
                setSelected([]);
              }}
            >
              {o.label}
            </Button>
          ))}
          <Button
            size="sm"
            variant="outline"
            onClick={() => void refreshCategories(true)}
            disabled={refreshing}
            className="ml-auto"
          >
            <RefreshCw className={`h-3.5 w-3.5 ${refreshing ? "animate-spin" : ""}`} />
            Refresh
          </Button>
          <Button
            size="sm"
            variant={combining ? "default" : "outline"}
            onClick={() => {
              setCombining((v) => !v);
              setSelected([]);
            }}
            title="Select several categories and browse videos in all of them"
          >
            <Layers className="h-3.5 w-3.5" />
            Combine
          </Button>
        </div>
        {combining && (
          <div className="space-y-2 rounded-md border border-[var(--color-border)] p-2">
            <p className="text-xs text-[var(--color-muted-foreground)]">
              {exact
                ? "Pick 2 — PornHub shows videos in both categories."
                : `Pick up to ${maxPick} — combined categories run as a word search for this orientation.`}
            </p>
            <div className="flex flex-wrap items-center gap-1.5">
              {selected.map((c) => (
                <button
                  key={c.slug}
                  type="button"
                  onClick={() => toggle(c)}
                  className="inline-flex items-center gap-1 rounded-full bg-[var(--color-primary)] px-3 py-1 text-xs text-[var(--color-primary-foreground)]"
                >
                  {c.name}
                  <X className="h-3 w-3" />
                </button>
              ))}
              <Button
                size="sm"
                className="ml-auto"
                disabled={selected.length === 0}
                onClick={() =>
                  void navigate({
                    to: "/browse/$site/$kind/$slug",
                    params: {
                      site: "pornhub",
                      kind: "category",
                      slug: selected.map((c) => c.slug).join(","),
                    },
                    search: { orientation },
                  })
                }
              >
                Browse {selected.length > 1 ? `${selected.length} together` : ""}
              </Button>
            </div>
          </div>
        )}
        {refreshStatus && (
          <p className="text-xs text-[var(--color-muted-foreground)]">{refreshStatus}</p>
        )}
        {orientation === "straight" && !combining && (
          <div className="space-y-1">
            <p className="text-xs text-[var(--color-muted-foreground)]">Language in videos</p>
            <div className="flex gap-1.5 overflow-x-auto pb-1 scrollbar-hide">
              {PORNHUB_LANGUAGES.map((l) => (
                <Link
                  key={l.slug}
                  to="/browse/$site/$kind/$slug"
                  params={{ site: "pornhub", kind: "category", slug: l.slug }}
                  className="shrink-0 rounded-full bg-[var(--color-secondary)] px-3 py-1 text-xs hover:bg-[var(--color-muted)]"
                >
                  {l.name}
                </Link>
              ))}
            </div>
          </div>
        )}
        <Input
          placeholder="Filter categories…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        />
        <ul className="max-h-[60vh] overflow-y-auto rounded-md border border-[var(--color-border)] divide-y divide-[var(--color-border)]">
          {categories.map((cat) => (
            <li key={`${cat.orientation}-${cat.slug}-${cat.categoryId ?? ""}`}>
              {combining ? (
                <button
                  type="button"
                  onClick={() => toggle(cat)}
                  className={`flex w-full items-center justify-between px-3 py-2 text-left text-sm hover:bg-[var(--color-muted)] ${
                    selected.some((c) => c.slug === cat.slug)
                      ? "bg-[var(--color-muted)] font-medium"
                      : ""
                  }`}
                >
                  <span>{cat.name}</span>
                  {cat.videoCount != null && (
                    <span className="text-xs text-[var(--color-muted-foreground)]">
                      {cat.videoCount.toLocaleString()}
                    </span>
                  )}
                </button>
              ) : (
                <Link
                  to="/browse/$site/$kind/$slug"
                  params={{ site: "pornhub", kind: "category", slug: categoryBrowseSlug(cat) }}
                  search={{ orientation: cat.orientation }}
                  className="flex w-full items-center justify-between px-3 py-2 text-left text-sm hover:bg-[var(--color-muted)]"
                >
                  <span>{cat.name}</span>
                  {cat.videoCount != null && (
                    <span className="text-xs text-[var(--color-muted-foreground)]">
                      {cat.videoCount.toLocaleString()}
                    </span>
                  )}
                </Link>
              )}
            </li>
          ))}
        </ul>
      </CardContent>
    </Card>
  );
}
