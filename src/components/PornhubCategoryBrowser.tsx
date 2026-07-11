import { useCallback, useMemo, useState } from "react";
import { api } from "@/lib/api/client";
import {
  categoriesForOrientation,
  categoryBrowseSlug,
  mergeCategoryCatalog,
  PORNHUB_CATEGORIES,
  PORNHUB_ORIENTATIONS,
  type BrowseOrientation,
  type PornhubCategory,
} from "@/lib/sites/pornhub-categories";
import type { MediaItem } from "@/lib/types";
import { SceneCard } from "@/components/SceneCard";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { RefreshCw } from "lucide-react";

export function PornhubCategoryBrowser() {
  const [orientation, setOrientation] = useState<BrowseOrientation>("straight");
  const [filter, setFilter] = useState("");
  const [selected, setSelected] = useState<PornhubCategory | null>(null);
  const [items, setItems] = useState<MediaItem[]>([]);
  const [page, setPage] = useState(1);
  const [hasMore, setHasMore] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [liveCatalog, setLiveCatalog] = useState<PornhubCategory[] | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [refreshStatus, setRefreshStatus] = useState("");

  const categories = useMemo(() => {
    const base = liveCatalog ?? categoriesForOrientation(orientation);
    const list = base.filter((c) => c.orientation === orientation);
    const q = filter.trim().toLowerCase();
    if (!q) return list;
    return list.filter((c) => c.name.toLowerCase().includes(q));
  }, [orientation, filter, liveCatalog]);

  const load = useCallback(async (cat: PornhubCategory, p: number, append = false) => {
    setLoading(true);
    setError("");
    try {
      const slug = categoryBrowseSlug(cat);
      const result = await api.browse("pornhub", "category", slug, p, cat.orientation);
      setItems((prev) => (append ? [...prev, ...result.items] : result.items));
      setHasMore(result.has_more);
      setPage(p);
      setSelected(cat);
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Browse failed";
      setError(msg.replace(/^site error:\s*/i, ""));
      if (!append) setItems([]);
    } finally {
      setLoading(false);
    }
  }, []);

  async function refreshCategories() {
    setRefreshing(true);
    setRefreshStatus("");
    try {
      const live = await api.listPornhubCategories(orientation);
      const staticForOrientation = PORNHUB_CATEGORIES.filter((c) => c.orientation === orientation);
      const merged = mergeCategoryCatalog(staticForOrientation, live);
      setLiveCatalog((prev) => {
        const others = (prev ?? PORNHUB_CATEGORIES).filter((c) => c.orientation !== orientation);
        return [...others, ...merged];
      });
      setRefreshStatus(`Updated ${merged.length} categories from PornHub.`);
    } catch (e) {
      setRefreshStatus(e instanceof Error ? e.message : "Refresh failed — import cookies first.");
    } finally {
      setRefreshing(false);
    }
  }

  async function handleDownload(item: MediaItem) {
    await api.queueDownload(item.url, "pornhub");
  }

  return (
    <div className="space-y-4">
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
                  setSelected(null);
                  setItems([]);
                  setFilter("");
                }}
              >
                {o.label}
              </Button>
            ))}
            <Button
              size="sm"
              variant="outline"
              onClick={() => void refreshCategories()}
              disabled={refreshing}
              className="ml-auto"
            >
              <RefreshCw className={`h-3.5 w-3.5 ${refreshing ? "animate-spin" : ""}`} />
              Refresh counts
            </Button>
          </div>
          {refreshStatus && (
            <p className="text-xs text-[var(--color-muted-foreground)]">{refreshStatus}</p>
          )}
          <Input
            placeholder="Filter categories…"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
          />
          <ul className="max-h-48 overflow-y-auto rounded-md border border-[var(--color-border)] divide-y divide-[var(--color-border)]">
            {categories.map((cat) => (
              <li key={`${cat.orientation}-${cat.slug}-${cat.categoryId ?? ""}`}>
                <button
                  type="button"
                  className="flex w-full items-center justify-between px-3 py-2 text-left text-sm hover:bg-[var(--color-muted)]"
                  onClick={() => void load(cat, 1)}
                >
                  <span>{cat.name}</span>
                  {cat.videoCount != null && (
                    <span className="text-xs text-[var(--color-muted-foreground)]">
                      {cat.videoCount.toLocaleString()}
                    </span>
                  )}
                </button>
              </li>
            ))}
          </ul>
        </CardContent>
      </Card>

      {selected && (
        <p className="text-sm text-[var(--color-muted-foreground)]">
          Showing: <strong>{selected.name}</strong> ({orientation})
        </p>
      )}

      {error && (
        <p className="rounded-md border border-red-400/30 bg-red-400/10 px-3 py-2 text-sm text-red-400">
          {error}
        </p>
      )}

      <div className="grid grid-cols-2 gap-3 md:grid-cols-3 lg:grid-cols-4">
        {items.map((item) => (
          <SceneCard key={item.id} item={item} onDownload={(i) => void handleDownload(i)} />
        ))}
      </div>

      {hasMore && selected && (
        <Button
          variant="outline"
          onClick={() => void load(selected, page + 1, true)}
          disabled={loading}
        >
          Load more
        </Button>
      )}

      {!loading && selected && items.length === 0 && (
        <p className="text-sm text-[var(--color-muted-foreground)]">
          No videos — import PornHub cookies in Settings.
        </p>
      )}
    </div>
  );
}
