import { useCallback, useEffect, useMemo, useState } from "react";
import { Link, useNavigate } from "@tanstack/react-router";
import { Check, Layers, RefreshCw, Search } from "lucide-react";
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
import { CombineBar } from "@/components/CombineBar";

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
    <section className="space-y-3">
      <div className="flex items-center gap-2">
        <h3 className="min-w-0 flex-1 truncate text-base font-semibold">
          Categories
          <span className="ml-2 text-xs font-normal text-[var(--color-muted-foreground)]">
            {categories.length}
          </span>
        </h3>
        <Button
          size="sm"
          variant={combining ? "default" : "outline"}
          onClick={() => {
            setCombining((v) => !v);
            setSelected([]);
          }}
          title="Select several categories and browse videos in all of them"
        >
          <Layers className="h-4 w-4" />
          Combine
        </Button>
        <Button
          size="sm"
          variant="ghost"
          onClick={() => void refreshCategories(true)}
          disabled={refreshing}
          title="Reload from PornHub"
          aria-label="Reload categories"
        >
          <RefreshCw className={`h-4 w-4 ${refreshing ? "animate-spin" : ""}`} />
        </Button>
      </div>

      <div className="grid grid-cols-4 gap-1.5 sm:flex">
        {PORNHUB_ORIENTATIONS.map((o) => (
          <button
            key={o.id}
            type="button"
            onClick={() => {
              setOrientation(o.id);
              setFilter("");
              setSelected([]);
            }}
            className={`rounded-lg px-3 py-2 text-sm font-medium transition ${
              orientation === o.id
                ? "bg-[var(--color-primary)] text-[var(--color-primary-foreground)]"
                : "bg-[var(--color-secondary)] hover:bg-[var(--color-muted)]"
            }`}
          >
            {o.label}
          </button>
        ))}
      </div>

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
                className="shrink-0 rounded-full bg-[var(--color-secondary)] px-3 py-1.5 text-xs hover:bg-[var(--color-muted)]"
              >
                {l.name}
              </Link>
            ))}
          </div>
        </div>
      )}

      <div className="relative">
        <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-[var(--color-muted-foreground)]" />
        <Input
          className="h-9 pl-8"
          placeholder="Find a category…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        />
      </div>

      <ul
        className={`overflow-hidden rounded-xl border border-[var(--color-border)] divide-y divide-[var(--color-border)] ${combining ? "mb-24" : ""}`}
      >
        {categories.map((cat) => {
          const picked = selected.some((c) => c.slug === cat.slug);
          const row = (
            <>
              <span className="flex min-w-0 items-center gap-2">
                {combining && (
                  <span
                    className={`flex h-5 w-5 shrink-0 items-center justify-center rounded border ${
                      picked
                        ? "border-[var(--color-primary)] bg-[var(--color-primary)] text-[var(--color-primary-foreground)]"
                        : "border-[var(--color-border)]"
                    }`}
                  >
                    {picked && <Check className="h-3.5 w-3.5" />}
                  </span>
                )}
                <span className="truncate">{cat.name}</span>
              </span>
              {cat.videoCount != null && (
                <span className="shrink-0 text-xs tabular-nums text-[var(--color-muted-foreground)]">
                  {cat.videoCount.toLocaleString()}
                </span>
              )}
            </>
          );
          const rowClass =
            "flex min-h-12 w-full items-center justify-between gap-3 px-3 text-left text-sm hover:bg-[var(--color-muted)] active:bg-[var(--color-muted)]";
          return (
            <li key={`${cat.orientation}-${cat.slug}-${cat.categoryId ?? ""}`}>
              {combining ? (
                <button type="button" onClick={() => toggle(cat)} className={rowClass}>
                  {row}
                </button>
              ) : (
                <Link
                  to="/browse/$site/$kind/$slug"
                  params={{ site: "pornhub", kind: "category", slug: categoryBrowseSlug(cat) }}
                  search={{ orientation: cat.orientation }}
                  className={rowClass}
                >
                  {row}
                </Link>
              )}
            </li>
          );
        })}
      </ul>

      {combining && (
        <CombineBar
          selected={selected.map((c) => ({ key: c.slug, label: c.name }))}
          hint={
            exact
              ? "Tap 2 categories — PornHub shows videos in both."
              : `Tap up to ${maxPick} — combined categories run as a word search here.`
          }
          onRemove={(slug) => setSelected((prev) => prev.filter((c) => c.slug !== slug))}
          onBrowse={() =>
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
        />
      )}
    </section>
  );
}
