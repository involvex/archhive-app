import { createFileRoute, Link, useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "react-hot-toast";
import { ArrowLeft, Globe, Minus, Plus, RefreshCw, Square, Tags, Trash2 } from "lucide-react";
import { api } from "@/lib/api/client";
import type {
  MediaItem,
  OnlineItem,
  OnlineQuery,
  OnlineSearchInfo,
  OnlineTagCount,
  SavedSearch,
} from "@/lib/types";
import { SceneCard, type CardItem } from "@/components/SceneCard";
import { BrowseItemDetailsDialog } from "@/components/BrowseItemDetailsDialog";
import { UrlPlayerDialog } from "@/components/UrlPlayerDialog";
import { EmptyState } from "@/components/EmptyState";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useInfiniteScroll } from "@/lib/hooks/useInfiniteScroll";

export const Route = createFileRoute("/library/online/")({
  component: OnlineLibraryPage,
  validateSearch: (s: Record<string, unknown>): { search?: string } =>
    typeof s.search === "string" && s.search ? { search: s.search } : {},
});

const PAGE_SIZE = 60;
/** Items per "Fetch tags" request (each is one page load on the source site). */
const DETAILS_BATCH = 5;

const DURATIONS: { id: string; label: string; min?: number; max?: number }[] = [
  { id: "", label: "Any length" },
  { id: "-10", label: "Under 10 min", max: 600 },
  { id: "10", label: "10+ min", min: 600 },
  { id: "20", label: "20+ min", min: 1200 },
  { id: "30", label: "30+ min", min: 1800 },
  { id: "10-30", label: "10–30 min", min: 600, max: 1800 },
];

const SORTS: { id: NonNullable<OnlineQuery["sort"]>; label: string; needsSearch?: boolean }[] = [
  { id: "newest", label: "Newest first" },
  { id: "site", label: "Site order (last update)", needsSearch: true },
  { id: "longest", label: "Longest" },
  { id: "shortest", label: "Shortest" },
  { id: "title", label: "Title A–Z" },
];

function toMediaItem(i: OnlineItem): MediaItem {
  return {
    id: i.key,
    title: i.title,
    url: i.url,
    thumbnail: i.thumbnail,
    duration: i.duration,
    site_id: i.site_id,
    performers: i.performers,
    tags: i.tags,
    channel: i.channel,
  };
}

function relative(iso?: string): string {
  if (!iso) return "never";
  const mins = Math.max(0, Math.floor((Date.now() - new Date(iso).getTime()) / 60000));
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins} min ago`;
  const h = Math.floor(mins / 60);
  if (h < 24) return `${h} h ago`;
  return `${Math.floor(h / 24)} d ago`;
}

/**
 * Online library: every result ArcHive has listed from a site, kept on the
 * device. Saved searches are browsed here from that local copy; only
 * "Update" (a few pages, slowly) and "Fetch tags" ask the sites.
 */
function OnlineLibraryPage() {
  const { search: searchParam } = Route.useSearch();
  const navigate = useNavigate();
  const searchId = searchParam ?? "";

  const [searches, setSearches] = useState<SavedSearch[]>([]);
  const [info, setInfo] = useState<OnlineSearchInfo[]>([]);
  const [text, setText] = useState("");
  const [debouncedText, setDebouncedText] = useState("");
  const [duration, setDuration] = useState("");
  // Newest found first; Update fetches the site's newest videos.
  const [sort, setSort] = useState<NonNullable<OnlineQuery["sort"]>>("newest");
  const [include, setInclude] = useState<string[]>([]);
  const [exclude, setExclude] = useState<string[]>([]);
  const [tagFind, setTagFind] = useState("");
  const [tagsOpen, setTagsOpen] = useState(false);
  const [tagCounts, setTagCounts] = useState<OnlineTagCount[]>([]);

  const [items, setItems] = useState<OnlineItem[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [pages, setPages] = useState(3);
  const [updating, setUpdating] = useState(false);
  const [fetching, setFetching] = useState<{ done: number; todo: number } | null>(null);
  const stopFetch = useRef(false);
  const tried = useRef(new Set<string>());

  const [infoItem, setInfoItem] = useState<MediaItem | null>(null);
  const [watchItem, setWatchItem] = useState<MediaItem | null>(null);

  const search = searches.find((s) => s.id === searchId);
  const searchInfo = info.find((i) => i.search_id === searchId);

  const loadMeta = useCallback(async () => {
    const [s, i] = await Promise.all([
      api.listSavedSearches().catch(() => [] as SavedSearch[]),
      api.onlineSearchInfo().catch(() => [] as OnlineSearchInfo[]),
    ]);
    setSearches(s);
    setInfo(i);
  }, []);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void loadMeta();
  }, [loadMeta]);

  useEffect(() => {
    const t = window.setTimeout(() => setDebouncedText(text), 300);
    return () => window.clearTimeout(t);
  }, [text]);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setSort("newest");
    setInclude([]);
    setExclude([]);
  }, [searchId]);

  const query = useMemo((): OnlineQuery => {
    const d = DURATIONS.find((x) => x.id === duration);
    return {
      search_id: searchId || undefined,
      text: debouncedText.trim() || undefined,
      include_tags: include,
      exclude_tags: exclude,
      min_duration: d?.min,
      max_duration: d?.max,
      sort,
      limit: PAGE_SIZE,
    };
  }, [searchId, debouncedText, include, exclude, duration, sort]);

  const loadPage = useCallback(
    async (offset: number) => {
      setLoading(true);
      try {
        const page = await api.queryOnlineItems({ ...query, offset });
        setItems((prev) => (offset === 0 ? page.items : [...prev, ...page.items]));
        setTotal(page.total);
      } catch (e) {
        toast.error(String(e));
      } finally {
        setLoading(false);
      }
    },
    [query],
  );

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void loadPage(0);
  }, [loadPage]);

  const loadTags = useCallback(async () => {
    setTagCounts(await api.onlineTagCounts(searchId || undefined).catch(() => []));
  }, [searchId]);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    if (tagsOpen) void loadTags();
  }, [tagsOpen, loadTags]);

  const hasMore = items.length < total;
  const loadMore = useCallback(() => {
    if (!loading && hasMore) void loadPage(items.length);
  }, [loading, hasMore, items.length, loadPage]);
  const { sentinelRef } = useInfiniteScroll(loadMore, hasMore, loading);

  async function runUpdate() {
    if (!searchId || updating) return;
    setUpdating(true);
    try {
      const r = await api.updateOnlineSearch(searchId, pages);
      const why =
        r.stopped === "caught up"
          ? " — caught up with the last update"
          : r.stopped === "end of results"
            ? " — end of the results"
            : r.stopped
              ? ` (stopped: ${r.stopped})`
              : "";
      toast.success(
        `Fetched ${r.pages} ${r.pages === 1 ? "page" : "pages"}, newest first: ${r.new_items} new, ${r.total} in total${why}`,
      );
      await Promise.all([loadMeta(), loadPage(0)]);
      if (tagsOpen) void loadTags();
    } catch (e) {
      toast.error(`Update failed: ${String(e)}`);
    } finally {
      setUpdating(false);
    }
  }

  async function fetchTags() {
    const todo = items.filter((i) => !i.details_at && !tried.current.has(i.key));
    if (todo.length === 0) {
      toast("Every listed video already has its tags fetched.");
      return;
    }
    stopFetch.current = false;
    setFetching({ done: 0, todo: todo.length });
    for (let i = 0; i < todo.length && !stopFetch.current; i += DETAILS_BATCH) {
      const batch = todo.slice(i, i + DETAILS_BATCH);
      batch.forEach((b) => tried.current.add(b.key));
      try {
        const updated = await api.fetchOnlineDetails(batch.map((b) => b.key));
        const byKey = new Map(updated.map((u) => [u.key, u]));
        setItems((prev) => prev.map((p) => byKey.get(p.key) ?? p));
      } catch (e) {
        toast.error(String(e));
        break;
      }
      setFetching({ done: Math.min(i + DETAILS_BATCH, todo.length), todo: todo.length });
    }
    setFetching(null);
    if (tagsOpen) void loadTags();
  }

  async function clearHistory() {
    if (!window.confirm("Remove browsing history that isn't part of a saved search?")) return;
    const removed = await api.clearOnlineHistory();
    toast.success(`Removed ${removed.toLocaleString()} videos`);
    await Promise.all([loadMeta(), loadPage(0)]);
  }

  function cycleTag(tag: string) {
    if (include.includes(tag)) {
      setInclude(include.filter((t) => t !== tag));
      setExclude([...exclude, tag]);
    } else if (exclude.includes(tag)) {
      setExclude(exclude.filter((t) => t !== tag));
    } else {
      setInclude([...include, tag]);
    }
  }

  const visibleTags = useMemo(() => {
    const q = tagFind.trim().toLowerCase();
    const chosen = new Set([...include, ...exclude]);
    const list = tagCounts.filter((t) => !q || t.tag.includes(q) || chosen.has(t.tag));
    // A typed word that isn't a tag yet can still filter titles.
    if (q && !list.some((t) => t.tag === q)) list.unshift({ tag: q, count: 0 });
    return list.slice(0, 200);
  }, [tagCounts, tagFind, include, exclude]);

  const handleCardClick = useCallback((item: CardItem) => {
    if ("site_id" in item) setWatchItem(item);
  }, []);
  const media = useMemo(() => items.map(toMediaItem), [items]);
  const missingDetails = items.filter((i) => !i.details_at).length;

  const chip = (state: "in" | "out" | "none") =>
    `inline-flex items-center gap-1 rounded-full px-2.5 py-1 text-xs transition ${
      state === "in"
        ? "bg-emerald-600 text-white"
        : state === "out"
          ? "bg-red-600 text-white line-through"
          : "bg-[var(--color-secondary)] hover:bg-[var(--color-muted)]"
    }`;

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <Link
          to="/library"
          aria-label="Library"
          className="-ml-1.5 shrink-0 rounded-full p-1.5 text-[var(--color-muted-foreground)] hover:bg-[var(--color-muted)]"
        >
          <ArrowLeft className="h-5 w-5" />
        </Link>
        <div className="min-w-0 flex-1">
          <h2 className="truncate text-lg font-bold md:text-2xl">Online library</h2>
          <p className="truncate text-xs text-[var(--color-muted-foreground)]">
            Results kept on this device — filtering never asks the site
          </p>
        </div>
      </div>

      <select
        className="h-10 w-full rounded-md border border-[var(--color-border)] bg-[var(--color-background)] px-3 text-sm md:max-w-md"
        value={searchId}
        onChange={(e) =>
          void navigate({
            to: "/library/online",
            search: e.target.value ? { search: e.target.value } : {},
          })
        }
        aria-label="Saved search"
      >
        <option value="">Everything browsed</option>
        {searches.map((s) => {
          const n = info.find((i) => i.search_id === s.id)?.item_count ?? 0;
          return (
            <option key={s.id} value={s.id}>
              {s.name} ({n.toLocaleString()})
            </option>
          );
        })}
      </select>

      {search ? (
        <div className="flex flex-wrap items-center gap-2 rounded-md border border-[var(--color-border)] p-3 text-xs">
          <span className="min-w-0 flex-1 text-[var(--color-muted-foreground)]">
            Updated {relative(searchInfo?.updated_at)} · {searchInfo?.item_count ?? 0} videos
          </span>
          <select
            className="h-8 rounded-md border border-[var(--color-border)] bg-[var(--color-background)] px-2"
            value={pages}
            onChange={(e) => setPages(Number(e.target.value))}
            disabled={updating}
            aria-label="Pages to fetch"
          >
            {[1, 3, 5, 10, 20].map((n) => (
              <option key={n} value={n}>
                {n} {n === 1 ? "page" : "pages"}
              </option>
            ))}
          </select>
          <Button size="sm" onClick={() => void runUpdate()} disabled={updating}>
            <RefreshCw className={`h-3.5 w-3.5 ${updating ? "animate-spin" : ""}`} />
            {updating ? "Updating slowly…" : "Update"}
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() =>
              void navigate({
                to: "/browse/$site/$kind/$slug",
                params: { site: search.site_id, kind: search.kind, slug: search.slug },
                search: { orientation: search.orientation, ...search.filters },
              })
            }
          >
            <Globe className="h-3.5 w-3.5" />
            Live
          </Button>
        </div>
      ) : (
        <div className="flex items-center gap-2 text-xs text-[var(--color-muted-foreground)]">
          <span className="flex-1">
            Everything ArcHive listed while you browsed. Save a search to update it here.
          </span>
          <Button size="sm" variant="ghost" onClick={() => void clearHistory()}>
            <Trash2 className="h-3.5 w-3.5" />
            Clear history
          </Button>
        </div>
      )}

      <div className="space-y-2">
        <Input
          placeholder="Search title, channel, performers, tags…"
          value={text}
          onChange={(e) => setText(e.target.value)}
        />
        <div className="flex flex-wrap gap-1.5">
          {DURATIONS.map((d) => (
            <button
              key={d.id}
              type="button"
              onClick={() => setDuration(d.id)}
              className={`rounded-full px-3 py-1 text-xs transition ${
                duration === d.id
                  ? "bg-[var(--color-primary)] text-[var(--color-primary-foreground)]"
                  : "bg-[var(--color-secondary)] hover:bg-[var(--color-muted)]"
              }`}
            >
              {d.label}
            </button>
          ))}
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <select
            className="h-8 rounded-md border border-[var(--color-border)] bg-[var(--color-background)] px-2 text-xs"
            value={sort}
            onChange={(e) => setSort(e.target.value as NonNullable<OnlineQuery["sort"]>)}
            aria-label="Sort"
          >
            {SORTS.filter((s) => !s.needsSearch || searchId).map((s) => (
              <option key={s.id} value={s.id}>
                {s.label}
              </option>
            ))}
          </select>
          <Button
            size="sm"
            variant={tagsOpen || include.length || exclude.length ? "default" : "outline"}
            onClick={() => setTagsOpen((v) => !v)}
          >
            <Tags className="h-3.5 w-3.5" />
            Tags
            {include.length + exclude.length > 0 && ` (${include.length + exclude.length})`}
          </Button>
          {fetching ? (
            <Button size="sm" variant="outline" onClick={() => (stopFetch.current = true)}>
              <Square className="h-3.5 w-3.5" />
              Stop ({fetching.done}/{fetching.todo})
            </Button>
          ) : (
            <Button
              size="sm"
              variant="outline"
              onClick={() => void fetchTags()}
              disabled={missingDetails === 0}
              title="Read tags, performers and channel from each listed video's own page (slow, a few at a time)"
            >
              Fetch tags{missingDetails > 0 ? ` (${missingDetails})` : ""}
            </Button>
          )}
        </div>

        {tagsOpen && (
          <div className="space-y-2 rounded-md border border-[var(--color-border)] p-2">
            <p className="text-[11px] text-[var(--color-muted-foreground)]">
              Tap once to require, twice to exclude, again to clear. A tag also matches titles that
              contain it, so it works before tags are fetched.
            </p>
            <Input
              className="h-8 text-xs"
              placeholder="Find or type a tag…"
              value={tagFind}
              onChange={(e) => setTagFind(e.target.value)}
            />
            <div className="flex max-h-56 flex-wrap gap-1.5 overflow-y-auto">
              {visibleTags.length === 0 && (
                <span className="text-xs text-[var(--color-muted-foreground)]">
                  No tags yet — use Fetch tags, or type one to match titles.
                </span>
              )}
              {visibleTags.map((t) => {
                const state = include.includes(t.tag)
                  ? "in"
                  : exclude.includes(t.tag)
                    ? "out"
                    : "none";
                return (
                  <button
                    key={t.tag}
                    type="button"
                    className={chip(state)}
                    onClick={() => cycleTag(t.tag)}
                  >
                    {state === "in" && <Plus className="h-3 w-3" />}
                    {state === "out" && <Minus className="h-3 w-3" />}
                    {t.tag}
                    {t.count > 0 && <span className="opacity-60">{t.count}</span>}
                  </button>
                );
              })}
            </div>
          </div>
        )}
      </div>

      <p className="text-xs text-[var(--color-muted-foreground)]">
        {total.toLocaleString()} {total === 1 ? "video" : "videos"}
      </p>

      {items.length === 0 && !loading ? (
        <EmptyState
          icon={<Globe className="h-8 w-8" />}
          title={search && !searchInfo?.updated_at ? "Not fetched yet" : "Nothing here"}
          description={
            search && !searchInfo?.updated_at
              ? "Tap Update to fetch the first pages of this saved search."
              : "Nothing matches these filters."
          }
        />
      ) : (
        <div className="grid grid-cols-2 gap-2.5 sm:grid-cols-3 md:grid-cols-4 md:gap-3 lg:grid-cols-5 xl:grid-cols-6">
          {media.map((item) => (
            <SceneCard
              key={item.id}
              item={item}
              compactActions
              onDownload={(i) =>
                void api
                  .queueDownload(i.url, i.site_id, i.title)
                  .then(() => toast.success("Added to downloads"))
                  .catch((e) => toast.error(String(e)))
              }
              onInfo={setInfoItem}
              onWatch={setWatchItem}
              onClick={handleCardClick}
            />
          ))}
        </div>
      )}
      {hasMore && <div ref={sentinelRef} className="h-4" />}

      <BrowseItemDetailsDialog
        item={infoItem}
        open={infoItem !== null}
        onClose={() => setInfoItem(null)}
      />
      <UrlPlayerDialog
        item={watchItem}
        playlist={media}
        open={watchItem !== null}
        onClose={() => setWatchItem(null)}
        onSelectItem={setWatchItem}
      />
    </div>
  );
}
