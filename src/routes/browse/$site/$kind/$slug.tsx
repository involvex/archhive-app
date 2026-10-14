import { createFileRoute, Link, useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowLeft,
  Bookmark,
  BookmarkCheck,
  CloudDownload,
  EyeOff,
  LayoutGrid,
  List,
  Search,
  SlidersHorizontal,
} from "lucide-react";
import { api, browseFiltersKey } from "@/lib/api/client";
import { useSiteInfo } from "@/lib/sites/useSiteInfo";
import { siteHasCategories } from "@/lib/sites/catalog";
import { groupForText, loadSiteCategories } from "@/components/SiteCategoryBrowser";
import { CategoryIncludeExclude } from "@/components/CategoryIncludeExclude";
import { CategoryTagNarrow } from "@/components/CategoryTagNarrow";
import { SuggestInput, type Suggestion } from "@/components/SuggestInput";
import { SiteAvatar } from "@/components/SitePicker";
import { BottomSheet } from "@/components/ui/bottom-sheet";
import { useMediaQuery } from "@/lib/hooks/useMediaQuery";
import {
  applyLocalFilters,
  BrowseFilterBar,
  EMPTY_LOCAL_FILTERS,
  LOCAL_DURATIONS,
  type LocalFilters,
} from "@/components/BrowseFilterBar";
import { normalizeBrowseInput } from "@/lib/browse/normalize";
import { filtersFromSearch, parseBrowseSearch } from "@/lib/browse/searchParams";
import { toast } from "react-hot-toast";
import type {
  BrowseFilters,
  BrowseKind,
  BrowseOrientation,
  FilterOption,
  MediaItem,
  SiteCategory,
} from "@/lib/types";
import { SceneCard, type CardItem } from "@/components/SceneCard";
import { BrowseItemDetailsDialog } from "@/components/BrowseItemDetailsDialog";
import { UrlPlayerDialog } from "@/components/UrlPlayerDialog";
import { SkeletonGrid } from "@/components/SkeletonGrid";
import { ErrorState } from "@/components/ErrorState";
import { EmptyState } from "@/components/EmptyState";
import { Button } from "@/components/ui/button";
import { useInfiniteScroll } from "@/lib/hooks/useInfiniteScroll";
import { usePullToRefresh } from "@/lib/hooks/usePullToRefresh";
import { browseCacheKey, useBrowseStore } from "@/lib/stores/browse";
import { PORNHUB_CATEGORIES } from "@/lib/sites/pornhub-categories";

export const Route = createFileRoute("/browse/$site/$kind/$slug")({
  component: BrowseDetailPage,
  // Orientation plus site-side filters, so saved searches reopen as saved.
  validateSearch: parseBrowseSearch,
});

function errorText(e: unknown): string {
  return e instanceof Error ? e.message : String(e ?? "");
}

/** A page adding fewer new videos than this share of the listing's usual
 * page size (at most MIN_NEW_PER_PAGE) is nearly empty: load the next one
 * too. Kept low: every extra page is more requests (three for an xPomelo
 * length range), and xPomelo rate limits after about 15. */
const MIN_NEW_SHARE = 0.2;
const MIN_NEW_PER_PAGE = 12;
/** Most extra pages fetched to top up one nearly empty page. */
const MAX_FILL_PAGES = 1;

/** Same video re-listed under another link: same title and length. */
function repeatKey(i: MediaItem): string | null {
  const title = i.title.toLowerCase().replace(/[^\p{L}\p{N}]/gu, "");
  return i.duration && title.length >= 8 ? `${title}|${i.duration}` : null;
}

/** `incoming` minus videos already in `prev` (by id, or title + length). */
function newItems(prev: MediaItem[], incoming: MediaItem[]): MediaItem[] {
  const ids = new Set(prev.map((i) => i.id));
  const keys = new Set(prev.map(repeatKey).filter(Boolean));
  return incoming.filter((i) => {
    const key = repeatKey(i);
    if (ids.has(i.id) || (key && keys.has(key))) return false;
    ids.add(i.id);
    if (key) keys.add(key);
    return true;
  });
}

function normalizeInitialSlug(rawSlug: string): string {
  if (rawSlug === "example") return "";
  if (rawSlug.startsWith("#")) return rawSlug.slice(1).trim();
  return rawSlug;
}

/** Turn legacy numeric category ids (e.g. "27") into readable slugs ("lesbian"). */
function normalizePornhubCategorySlug(raw: string): string {
  if (!/^\d+$/.test(raw)) return raw;
  const hit = PORNHUB_CATEGORIES.find((c) => String(c.categoryId) === raw);
  return hit?.slug ?? raw;
}

/** Pause in typing before asking the site for suggestions. */
const REMOTE_SUGGEST_DELAY_MS = 400;
/** Site suggestions already fetched this session (`site|orientation|text`). */
const REMOTE_SUGGEST_CACHE = new Map<string, string[]>();

function BrowseDetailPage() {
  const { site, kind, slug } = Route.useParams();
  const navigate = useNavigate();
  const cleanSlug = normalizeInitialSlug(slug);
  const browseSlug =
    site === "pornhub" && kind === "category" ? normalizePornhubCategorySlug(cleanSlug) : cleanSlug;
  const routeSearch = Route.useSearch();
  const initialOrientation = routeSearch.orientation;
  const initialFilters = useMemo(() => filtersFromSearch(routeSearch), [routeSearch]);
  const initialFiltersKey = browseFiltersKey(initialFilters);
  const [orientation, setOrientation] = useState<BrowseOrientation>(
    initialOrientation ?? "straight",
  );
  const isPornhubAnimal =
    (site === "pornhub" && (kind === "category" || kind === "tag" || kind === "search")) ||
    (site === "chaturbate" && (kind === "livestream" || kind === "tag" || kind === "search")) ||
    (site === "pornwitch" && (kind === "category" || kind === "search"));
  const ORIENT_TABS: { value: BrowseOrientation; label: string }[] = [
    { value: "straight", label: "Straight" },
    { value: "gay", label: "Gay" },
    { value: "lesbian", label: "Lesbian" },
    { value: "transgender", label: "Trans" },
  ];

  const cacheKey = useMemo(
    () =>
      browseCacheKey({
        site,
        kind,
        slug: `${browseSlug}${initialFiltersKey}`,
        orientation: initialOrientation,
      }),
    [site, kind, browseSlug, initialOrientation, initialFiltersKey],
  );
  const cached = useBrowseStore((s) => s.caches[cacheKey]);
  const setCache = useBrowseStore((s) => s.set);

  const [items, setItems] = useState<MediaItem[]>(cached?.items ?? []);
  const [page, setPage] = useState(cached?.page ?? 1);
  const [hasMore, setHasMore] = useState(cached?.hasMore ?? false);
  // Why the last "load more" failed; pauses infinite scroll until Retry.
  const [moreError, setMoreError] = useState("");
  // Total matches the site reports for this listing (xPomelo), if any.
  const [resultTotal, setResultTotal] = useState<number | null>(null);
  const [querySlug, setQuerySlug] = useState(cached?.querySlug ?? browseSlug);
  const siteInfo = useSiteInfo(site);
  const [filters, setFilters] = useState<BrowseFilters>(cached?.filters ?? initialFilters);
  const [localFilters, setLocalFilters] = useState<LocalFilters>(EMPTY_LOCAL_FILTERS);
  // Tag autocomplete from the site's tag index. Grouped indexes (XVIDEOS:
  // ~50k tags split by first letter) load only the group being typed.
  const [tagIndex, setTagIndex] = useState<SiteCategory[]>([]);
  const suggestTags =
    (kind === "tag" || kind === "category") &&
    site !== "pornhub" &&
    Boolean(siteInfo && siteHasCategories(siteInfo));
  const tagGroups = siteInfo?.browse_options?.category_groups ?? [];
  // Suggest for the tag being typed (after the last comma when combining).
  const typedTag = querySlug.split(",").pop()?.trim().toLowerCase() ?? "";
  // XVIDEOS keeps categories (/c/…) in their own group, separate from tags.
  // Category pages suggest categories: XVIDEOS keeps them in a "categories"
  // group, Eporner in its main index; tag pages use the typed letter's group.
  const categoryKind = kind === "category";
  // xPomelo's main index is only its popular tags; it names the full list.
  const fullTagGroup = siteInfo?.browse_options?.tag_index_group;
  const tagGroup = categoryKind
    ? tagGroups.some((g) => g.id === "categories")
      ? "categories"
      : fullTagGroup
    : groupForText(typedTag, tagGroups);
  const tagGroupKey = tagGroup ?? (categoryKind || tagGroups.length === 0 ? "all" : "");
  useEffect(() => {
    if (!suggestTags || tagGroupKey === "") return;
    let cancelled = false;
    loadSiteCategories(site, false, tagGroup)
      .then((cats) => {
        if (!cancelled) setTagIndex(cats);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [site, suggestTags, tagGroupKey]);
  // Sites that suggest phrases themselves (Pornwitch): ask once typing
  // pauses, for the tag being typed (an excluded "-tag" without its dash).
  const remoteSuggest = Boolean(siteInfo?.browse_options?.remote_suggest);
  const remoteText = typedTag.replace(/^-/, "").trim();
  const suggestOrientation = isPornhubAnimal ? orientation : undefined;
  const [remoteWords, setRemoteWords] = useState<{ key: string; words: string[] }>({
    key: "",
    words: [],
  });
  const remoteKey = `${site}|${suggestOrientation ?? ""}|${remoteText}`;
  useEffect(() => {
    if (!remoteSuggest || remoteText.length < 2) return;
    const hit = REMOTE_SUGGEST_CACHE.get(remoteKey);
    if (hit) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setRemoteWords({ key: remoteKey, words: hit });
      return;
    }
    let cancelled = false;
    const timer = window.setTimeout(() => {
      api
        .siteSuggest(site, remoteText, suggestOrientation)
        .then((words) => {
          REMOTE_SUGGEST_CACHE.set(remoteKey, words);
          if (!cancelled) setRemoteWords({ key: remoteKey, words });
        })
        .catch(() => {});
    }, REMOTE_SUGGEST_DELAY_MS);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [remoteSuggest, remoteKey]);
  const tagSuggestions = useMemo(() => {
    if (!typedTag) return [];
    const prefix = querySlug.includes(",")
      ? `${querySlug.slice(0, querySlug.lastIndexOf(",") + 1)}`
      : "";
    const dash = typedTag.startsWith("-") ? "-" : "";
    const remote: Suggestion[] =
      remoteWords.key === remoteKey
        ? remoteWords.words.map((w) => ({
            value: `${prefix}${dash}${w}`,
            label: `${dash}${w}`,
            hint: "suggested",
          }))
        : [];
    return remote.concat(
      tagIndex
        .filter(
          (c) =>
            c.slug.toLowerCase().startsWith(typedTag) || c.name.toLowerCase().startsWith(typedTag),
        )
        .sort((a, b) => (b.count ?? 0) - (a.count ?? 0))
        .slice(0, 50)
        .map((c): Suggestion => ({
          value: `${prefix}${c.slug}`,
          label: c.name,
          hint: c.count != null ? c.count.toLocaleString() : undefined,
        })),
    );
  }, [tagIndex, typedTag, querySlug, remoteWords, remoteKey]);
  const filtersKey = browseFiltersKey(filters);
  // Site filters are edited as a draft and only load results on Apply, so
  // changing several settings costs one request instead of one per tap.
  const [draftFilters, setDraftFilters] = useState<BrowseFilters>(filters);
  const draftKey = browseFiltersKey(draftFilters);
  const draftDirty = draftKey !== filtersKey;
  useEffect(() => {
    // Applied filters changed elsewhere (Apply, saved search): follow them.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setDraftFilters(filters);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filtersKey]);
  const applyFilters = useCallback(() => setFilters(draftFilters), [draftFilters]);
  const discardFilters = useCallback(() => setDraftFilters(filters), [filters]);
  const [loading, setLoading] = useState(false);
  const [initial, setInitial] = useState(!cached?.items?.length);
  const [error, setError] = useState("");
  const [fromCache, setFromCache] = useState(false);
  const [cacheAgeSecs, setCacheAgeSecs] = useState<number | null>(null);
  const [infoItem, setInfoItem] = useState<MediaItem | null>(null);
  const [watchItem, setWatchItem] = useState<MediaItem | null>(null);
  const [viewMode, setViewMode] = useState<"grid" | "list">("grid");
  // #28 saved search + hide-watched-matches.
  const [savedId, setSavedId] = useState<string | null>(null);
  const [savingSearch, setSavingSearch] = useState(false);
  const [hideWatched, setHideWatched] = useState(false);
  const [watchedUrls, setWatchedUrls] = useState<Set<string> | null>(null);
  // Tap-to-play: tapping a thumbnail opens the video player dialog.
  const handleCardClick = useCallback((item: CardItem) => {
    if ("site_id" in item) setWatchItem(item);
  }, []);

  useEffect(() => {
    setCache(cacheKey, { items, page, hasMore, querySlug, filters });
  }, [cacheKey, items, page, hasMore, querySlug, filters, setCache]);

  // The items as of the last render, for topping up short pages, and the
  // biggest page this listing returned (its usual page size).
  const itemsRef = useRef(items);
  const pageSizeRef = useRef(0);
  useEffect(() => {
    itemsRef.current = items;
  });

  const load = useCallback(
    async (p: number, append = false) => {
      if (!querySlug.trim()) return;
      setLoading(true);
      setError("");
      setMoreError("");
      let current = append ? itemsRef.current : [];
      if (!append) pageSizeRef.current = 0;
      let pageNo = p;
      try {
        const normalized = normalizeBrowseInput(site, kind as BrowseKind, querySlug.trim());
        // A page that adds only a few new videos (aggregators re-list the
        // same video under other links) is topped up from the next pages.
        for (let extra = 0; ; extra++) {
          const result = await api.browse(
            site,
            normalized.kind,
            normalized.slug,
            pageNo,
            isPornhubAnimal ? orientation : undefined,
            filters,
          );
          const fresh = newItems(current, result.items);
          pageSizeRef.current = Math.max(pageSizeRef.current, result.items.length);
          const enough = Math.min(MIN_NEW_PER_PAGE, Math.ceil(pageSizeRef.current * MIN_NEW_SHARE));
          current = [...current, ...fresh];
          setItems(current);
          setHasMore(result.has_more);
          if (!append && extra === 0) setResultTotal(result.total ?? null);
          setPage(pageNo);
          setFromCache(Boolean(result.from_cache));
          setCacheAgeSecs(result.cache_age_secs ?? null);
          setInitial(false);
          if (
            !result.has_more ||
            result.from_cache ||
            fresh.length >= enough ||
            extra >= MAX_FILL_PAGES
          ) {
            break;
          }
          pageNo += 1;
        }
      } catch (e) {
        // Tauri rejects with plain strings, not Error objects.
        const msg = (errorText(e) || "Browse failed").replace(/^site error:\s*/i, "");
        if (current.length === 0) {
          setError(msg);
          setItems([]);
          // Also as a toast: the page may be scrolled or behind a sheet.
          toast.error(msg, { id: "browse-error", duration: 8000 });
        } else {
          // Loading more failed (often a rate limit): say so where the
          // user is looking, and stop auto-loading until Retry.
          setMoreError(msg);
          toast.error(msg, { duration: 8000 });
        }
        console.error(e);
      } finally {
        setLoading(false);
      }
    },
    [site, kind, querySlug, orientation, isPornhubAnimal, filters],
  );

  // Changing a site-side sort/filter reloads from page 1. `load` is read via
  // a ref so typing in the slug box doesn't trigger reloads.
  const loadRef = useRef(load);
  useEffect(() => {
    loadRef.current = load;
  });
  // Opening a category / tag / saved search loads page 1 right away unless
  // results for it are already cached.
  const hasCachedItems = Boolean(cached?.items?.length);
  useEffect(() => {
    if (hasCachedItems || !browseSlug.trim()) return;
    void loadRef.current(1);
    // Only on first open of each listing.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [cacheKey]);

  const lastFiltersKey = useRef(filtersKey);
  useEffect(() => {
    if (lastFiltersKey.current === filtersKey) return;
    lastFiltersKey.current = filtersKey;
    setInitial(true);
    setItems([]);
    void loadRef.current(1);
  }, [filtersKey]);

  const loadMore = useCallback(() => {
    if (!loading && hasMore) {
      void load(page + 1, true);
    }
  }, [loading, hasMore, page, load]);

  const {
    sentinelRef,
    paused: autoLoadPaused,
    loadMore: loadMoreNow,
  } = useInfiniteScroll(loadMore, hasMore, loading);

  const handleRefresh = useCallback(async () => {
    void load(1, false);
  }, [load]);

  const { containerRef, pullDistance, refreshing } = usePullToRefresh({
    onRefresh: handleRefresh,
    disabled: false,
  });

  async function handleDownload(item: MediaItem) {
    try {
      // Aggregator results (xPomelo) carry the source site's id, so the
      // download goes through that site's adapter.
      await api.queueDownload(item.url, item.site_id || site, item.title);
      toast.success("Added to downloads");
    } catch (e) {
      toast.error(`Download failed: ${errorText(e)}`);
    }
  }

  // #28: detect whether this exact search (incl. sort / filters) is already saved.
  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setSavedId(null);
    void api
      .listSavedSearches()
      .then((all) => {
        const match = all.find(
          (s) =>
            s.site_id === site &&
            s.kind === (kind as BrowseKind) &&
            s.slug === browseSlug &&
            (s.orientation ?? undefined) === (isPornhubAnimal ? orientation : undefined) &&
            browseFiltersKey(s.filters) === filtersKey,
        );
        setSavedId(match?.id ?? null);
      })
      .catch(() => {});
  }, [site, kind, browseSlug, orientation, isPornhubAnimal, filtersKey]);

  /** Save this search (with its filters) if needed, fetch every page of it
   * into the Online library in the background, and open it there. */
  async function fetchAllPages() {
    if (savingSearch || !browseSlug) return;
    setSavingSearch(true);
    try {
      let id = savedId;
      if (!id) {
        const saved = await api.saveSearch({
          name: `${siteInfo?.display_name ?? site} ${kind}: ${browseSlug}${filtersKey ? " (filtered)" : ""}`,
          site_id: site,
          kind: kind as BrowseKind,
          slug: browseSlug,
          orientation: isPornhubAnimal ? orientation : undefined,
          filters: filtersKey ? filters : undefined,
        });
        id = saved.id;
        setSavedId(id);
      }
      await api.startFullFetch(id);
      toast.success(
        "Fetching every page in the background. It pauses by itself on rate limits and carries on afterwards.",
      );
      void navigate({ to: "/library/online", search: { search: id } });
    } catch (e) {
      toast.error(`Could not start: ${errorText(e)}`);
    } finally {
      setSavingSearch(false);
    }
  }

  async function toggleSavedSearch() {
    if (savingSearch) return;
    setSavingSearch(true);
    try {
      if (savedId) {
        await api.deleteSavedSearch(savedId);
        setSavedId(null);
      } else {
        const saved = await api.saveSearch({
          name: `${siteInfo?.display_name ?? site} ${kind}: ${browseSlug}${filtersKey ? " (filtered)" : ""}`,
          site_id: site,
          kind: kind as BrowseKind,
          slug: browseSlug,
          orientation: isPornhubAnimal ? orientation : undefined,
          filters: filtersKey ? filters : undefined,
        });
        setSavedId(saved.id);
        toast.success("Saved — find it under Browse → Saved searches");
      }
    } catch (e) {
      console.error(e);
      toast.error(`Could not ${savedId ? "remove" : "save"} search: ${errorText(e)}`);
    } finally {
      setSavingSearch(false);
    }
  }

  // Phones get a compact header and a Filters sheet; desktop keeps the
  // controls inline.
  const isPhone = useMediaQuery("(max-width: 767px)");
  const [filtersOpen, setFiltersOpen] = useState(false);
  const browseOptions = siteInfo?.browse_options;
  const activeFilterLabels = useMemo(() => {
    const label = (list: FilterOption[] | undefined, id?: string) =>
      id ? (list?.find((o) => o.id === id)?.label ?? id) : "";
    const count = (v?: string) => (v ? v.split(",").filter(Boolean).length : 0);
    return [
      label(browseOptions?.sorts, filters.sort),
      label(browseOptions?.periods, filters.period),
      label(browseOptions?.durations, filters.duration),
      label(browseOptions?.qualities, filters.quality),
      label(browseOptions?.sources, filters.source),
      count(filters.include) ? `${count(filters.include)} required` : "",
      count(filters.exclude) ? `${count(filters.exclude)} excluded` : "",
      filters.tag ? `#${filters.tag}` : "",
      localFilters.duration
        ? (LOCAL_DURATIONS.find((d) => d.id === localFilters.duration)?.label ?? "")
        : "",
      localFilters.text.trim() ? `“${localFilters.text.trim()}”` : "",
      hideWatched ? "Hide watched" : "",
    ].filter(Boolean);
  }, [browseOptions, filters, localFilters, hideWatched]);

  // #28: hide results already watched in the library (matched by source URL).
  async function toggleHideWatched() {
    const next = !hideWatched;
    setHideWatched(next);
    if (next && watchedUrls === null) {
      try {
        setWatchedUrls(new Set(await api.listWatchedSourceUrls()));
      } catch {
        setWatchedUrls(new Set());
      }
    }
  }

  const unwatched =
    hideWatched && watchedUrls ? items.filter((i) => !watchedUrls.has(i.url)) : items;
  const hiddenCount = items.length - unwatched.length;
  const visibleItems = applyLocalFilters(unwatched, localFilters);
  const filteredOut = unwatched.length - visibleItems.length;

  const showInitialSkeleton = initial && loading && !error;
  const showEmpty = !loading && !error && visibleItems.length === 0 && querySlug && !initial;
  const showItems = visibleItems.length > 0;

  const hideWatchedButton = (
    <Button
      size="sm"
      variant={hideWatched ? "default" : "outline"}
      onClick={() => void toggleHideWatched()}
      title="Hide results already watched in your library"
      aria-label="Hide watched"
      aria-pressed={hideWatched}
    >
      <EyeOff className="h-4 w-4" />
      <span className="hidden sm:inline">Hide watched</span>
    </Button>
  );

  const viewToggle = (
    <div className="flex shrink-0 overflow-hidden rounded-md border border-[var(--color-border)]">
      {(
        [
          ["grid", LayoutGrid, "Grid view"],
          ["list", List, "List view"],
        ] as const
      ).map(([mode, Icon, label]) => (
        <button
          key={mode}
          type="button"
          onClick={() => setViewMode(mode)}
          className={`p-2 transition-colors ${viewMode === mode ? "bg-[var(--color-muted)] text-[var(--color-foreground)]" : "text-[var(--color-muted-foreground)] hover:text-[var(--color-foreground)]"}`}
          aria-label={label}
          aria-pressed={viewMode === mode}
        >
          <Icon className="h-4 w-4" />
        </button>
      ))}
    </div>
  );

  const filterControls = (
    <div className="space-y-3">
      {isPornhubAnimal && (
        <div className="flex flex-wrap gap-1.5">
          {ORIENT_TABS.map((o) => (
            <button
              key={o.value}
              type="button"
              onClick={() => {
                setOrientation(o.value);
                setInitial(true);
                setItems([]);
                void load(1);
              }}
              className={`inline-flex items-center rounded-full px-3 py-1.5 text-xs font-medium transition ${
                orientation === o.value
                  ? "bg-[var(--color-primary)] text-[var(--color-primary-foreground)]"
                  : "bg-[var(--color-secondary)] hover:bg-[var(--color-muted)]"
              }`}
            >
              {o.label}
            </button>
          ))}
        </div>
      )}
      <BrowseFilterBar
        options={browseOptions}
        kind={kind as BrowseKind}
        filters={draftFilters}
        onFiltersChange={setDraftFilters}
        local={localFilters}
        onLocalChange={setLocalFilters}
      />
      {browseOptions?.include_exclude &&
        (browseOptions.include_exclude_kinds?.length
          ? browseOptions.include_exclude_kinds
          : browseOptions.filter_kinds
        ).includes(kind as BrowseKind) && (
          <CategoryIncludeExclude
            siteId={site}
            group={browseOptions.tag_index_group}
            filters={draftFilters}
            onFiltersChange={setDraftFilters}
            maxInclude={
              browseOptions.exclude_only_kinds?.includes(kind as BrowseKind)
                ? 0
                : browseOptions.max_include || Infinity
            }
            maxExclude={browseOptions.max_exclude || Infinity}
          />
        )}
      {kind === "category" && browseOptions?.category_tag_filter && (
        <CategoryTagNarrow
          siteId={site}
          groups={browseOptions.category_groups ?? []}
          filters={draftFilters}
          onFiltersChange={setDraftFilters}
        />
      )}
      {draftDirty && !isPhone && (
        <div className="flex items-center gap-2">
          <Button size="sm" onClick={applyFilters}>
            Apply filters
          </Button>
          <Button size="sm" variant="ghost" onClick={discardFilters}>
            Discard changes
          </Button>
        </div>
      )}
    </div>
  );

  return (
    <div ref={containerRef} className="space-y-4">
      {pullDistance > 0 && (
        <div
          className="flex items-center justify-center transition-height overflow-hidden"
          style={{ height: pullDistance }}
        >
          <div
            className={`text-xs transition-opacity ${pullDistance > 60 ? "text-[var(--color-primary)]" : "text-[var(--color-muted-foreground)]"}`}
          >
            {refreshing
              ? "Refreshing..."
              : pullDistance > 60
                ? "Release to refresh"
                : "Pull to refresh"}
          </div>
        </div>
      )}

      {fromCache && (
        <div
          className="rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-xs text-amber-200"
          role="status"
        >
          Cached
          {cacheAgeSecs != null
            ? ` · ${cacheAgeSecs < 60 ? `${cacheAgeSecs}s` : `${Math.round(cacheAgeSecs / 60)}m`} ago`
            : ""}{" "}
          — pull to refresh
        </div>
      )}

      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <Link
            to="/browse"
            aria-label="Back to Browse"
            className="-ml-1.5 shrink-0 rounded-full p-1.5 text-[var(--color-muted-foreground)] hover:bg-[var(--color-muted)] hover:text-[var(--color-foreground)]"
          >
            <ArrowLeft className="h-5 w-5" />
          </Link>
          {siteInfo && <SiteAvatar site={siteInfo} size="sm" />}
          <div className="min-w-0 flex-1">
            <h2 className="truncate text-lg font-bold leading-tight md:text-2xl">
              {siteInfo?.display_name ?? site}
            </h2>
            <p className="truncate text-xs capitalize text-[var(--color-muted-foreground)]">
              {kind}
              {isPornhubAnimal && orientation !== "straight"
                ? ` · ${ORIENT_TABS.find((o) => o.value === orientation)?.label}`
                : ""}
            </p>
          </div>
          <Button
            size="sm"
            variant={savedId ? "default" : "outline"}
            onClick={() => void toggleSavedSearch()}
            disabled={savingSearch || !browseSlug}
            title={savedId ? "Remove saved search" : "Save this search"}
            aria-label={savedId ? "Remove saved search" : "Save this search"}
          >
            {savedId ? <BookmarkCheck className="h-4 w-4" /> : <Bookmark className="h-4 w-4" />}
            <span className="hidden sm:inline">{savedId ? "Saved" : "Save"}</span>
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() => void fetchAllPages()}
            disabled={savingSearch || !browseSlug}
            title="Save this search and fetch all its pages into the Online library (slowly, pausing on rate limits)"
            aria-label="Fetch all pages into the Online library"
          >
            <CloudDownload className="h-4 w-4" />
            <span className="hidden sm:inline">Fetch all</span>
          </Button>
          {!isPhone && (
            <>
              {hideWatchedButton}
              {showItems && viewToggle}
            </>
          )}
        </div>

        <div className="flex gap-2 md:max-w-md">
          <SuggestInput
            placeholder={
              suggestTags ? `Type a ${kind} (comma-separate to combine)` : `Enter ${kind} slug...`
            }
            suggestions={suggestTags || remoteSuggest ? tagSuggestions : []}
            value={querySlug}
            onValueChange={(v) => setQuerySlug(v.startsWith("#") ? v.slice(1) : v)}
            className="h-10"
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                let val = querySlug;
                if (val.startsWith("#")) val = val.slice(1).trim();
                if (val) {
                  setQuerySlug(val);
                  setInitial(true);
                  setItems([]);
                  void load(1);
                }
              }
            }}
          />
          <Button
            className="h-10 shrink-0"
            onClick={() => {
              setInitial(true);
              setItems([]);
              void load(1);
            }}
            disabled={loading}
            aria-label="Browse"
          >
            <Search className="h-4 w-4 sm:hidden" />
            <span className="hidden sm:inline">Browse</span>
          </Button>
        </div>
        {querySlug.includes(",") &&
          (siteInfo?.browse_options?.combine_tags ?? 0) <
            querySlug.split(",").filter((t) => t.trim()).length && (
            <p className="text-xs text-[var(--color-muted-foreground)]">
              This site can&apos;t match all these tags exactly — results come from a word search.
            </p>
          )}

        {isPhone ? (
          <div className="flex items-center gap-2">
            <Button
              size="sm"
              variant={activeFilterLabels.length ? "default" : "outline"}
              className="h-9 shrink-0"
              onClick={() => setFiltersOpen(true)}
            >
              <SlidersHorizontal className="h-4 w-4" />
              Filters
              {activeFilterLabels.length > 0 && (
                <span className="ml-0.5 rounded-full bg-[var(--color-primary-foreground)]/20 px-1.5 text-[11px] tabular-nums">
                  {activeFilterLabels.length}
                </span>
              )}
            </Button>
            <div className="flex min-w-0 flex-1 gap-1.5 overflow-x-auto scrollbar-hide">
              {activeFilterLabels.length === 0 ? (
                <span className="truncate text-xs text-[var(--color-muted-foreground)]">
                  Site default order
                </span>
              ) : (
                activeFilterLabels.map((l) => (
                  <span
                    key={l}
                    className="shrink-0 rounded-full bg-[var(--color-secondary)] px-2.5 py-1 text-[11px]"
                  >
                    {l}
                  </span>
                ))
              )}
            </div>
            {draftDirty && (
              <Button size="sm" className="h-9 shrink-0" onClick={applyFilters}>
                Apply
              </Button>
            )}
            {showItems && viewToggle}
          </div>
        ) : (
          filterControls
        )}

        {resultTotal != null && resultTotal > 0 && (
          <p className="text-xs text-[var(--color-muted-foreground)]">
            {resultTotal.toLocaleString()} {resultTotal === 1 ? "video" : "videos"} on the site{" "}
            (counts repeat uploads and paysites, which are hidden here)
          </p>
        )}
        {(hiddenCount > 0 || filteredOut > 0) && (
          <p className="text-xs text-[var(--color-muted-foreground)]">
            {[
              hiddenCount > 0
                ? `${hiddenCount} watched ${hiddenCount === 1 ? "result" : "results"} hidden`
                : "",
              filteredOut > 0
                ? `${filteredOut} loaded ${filteredOut === 1 ? "result" : "results"} filtered out`
                : "",
            ]
              .filter(Boolean)
              .join(" · ")}
            .
          </p>
        )}
      </div>

      <BottomSheet
        open={filtersOpen}
        onClose={() => setFiltersOpen(false)}
        title="Filters"
        footer={
          <Button
            className="h-11 w-full"
            onClick={() => {
              if (draftDirty) applyFilters();
              setFiltersOpen(false);
            }}
          >
            {draftDirty ? "Apply filters" : "Show results"}
          </Button>
        }
      >
        <div className="space-y-4">
          {filterControls}
          <button
            type="button"
            role="switch"
            aria-checked={hideWatched}
            onClick={() => void toggleHideWatched()}
            className="flex w-full items-center justify-between gap-3 border-t border-[var(--color-border)] pt-3 text-left text-sm"
          >
            Hide videos already in my library
            <span
              className={`relative h-6 w-11 shrink-0 rounded-full transition-colors ${hideWatched ? "bg-[var(--color-primary)]" : "bg-[var(--color-muted)]"}`}
            >
              <span
                className={`absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition-all ${hideWatched ? "left-[1.375rem]" : "left-0.5"}`}
              />
            </span>
          </button>
        </div>
      </BottomSheet>

      {error && <ErrorState message={error} onRetry={() => void load(page, false)} />}

      {showInitialSkeleton && <SkeletonGrid count={12} cols={isPhone ? 2 : 3} />}

      {showItems && viewMode === "grid" && (
        <div className="grid grid-cols-2 gap-2.5 sm:grid-cols-3 md:grid-cols-4 md:gap-3 lg:grid-cols-5 xl:grid-cols-6">
          {visibleItems.map((item) => (
            <SceneCard
              key={item.id}
              item={item}
              compactActions={isPhone}
              onDownload={(i) => void handleDownload(i)}
              onInfo={setInfoItem}
              onWatch={setWatchItem}
              onClick={handleCardClick}
            />
          ))}
        </div>
      )}

      {showItems && viewMode === "list" && (
        <div className="flex flex-col gap-2">
          {visibleItems.map((item) => (
            <SceneCard
              key={item.id}
              item={item}
              listView
              onDownload={(i) => void handleDownload(i)}
              onInfo={setInfoItem}
              onWatch={setWatchItem}
              onClick={handleCardClick}
            />
          ))}
        </div>
      )}

      {showItems && hasMore && !moreError && <div ref={sentinelRef} className="h-4" />}
      {showItems && hasMore && !moreError && autoLoadPaused && !loading && (
        <div className="flex justify-center py-2">
          <Button variant="outline" size="sm" onClick={loadMoreNow}>
            Load more
          </Button>
        </div>
      )}

      {loading && showItems && (
        <div className="flex justify-center py-4 text-xs text-[var(--color-muted-foreground)]">
          Loading more...
        </div>
      )}
      {moreError && !loading && showItems && (
        <div className="flex flex-col items-center gap-2 rounded-md border border-[var(--color-border)] p-3 text-center text-xs">
          <p className="text-[var(--color-muted-foreground)]">Couldn't load more: {moreError}</p>
          <Button size="sm" variant="outline" onClick={() => void load(page + 1, true)}>
            Retry
          </Button>
        </div>
      )}

      {showEmpty && (
        <EmptyState
          icon={<Search className="h-8 w-8" />}
          title="No items found"
          description="Try a different search term or browse another category."
        />
      )}

      <BrowseItemDetailsDialog
        item={infoItem}
        open={infoItem !== null}
        onClose={() => setInfoItem(null)}
      />

      <UrlPlayerDialog
        item={watchItem}
        playlist={items}
        open={watchItem !== null}
        onClose={() => setWatchItem(null)}
        onSelectItem={setWatchItem}
      />
    </div>
  );
}
