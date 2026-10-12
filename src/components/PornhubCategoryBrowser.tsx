import { useMemo, useState } from "react";
import { Link } from "@tanstack/react-router";
import { RefreshCw } from "lucide-react";
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
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

/**
 * PornHub category picker. Selecting a category opens the full browse page
 * (sort / filters / infinite scroll) with the orientation carried in the URL.
 */
export function PornhubCategoryBrowser() {
  const [orientation, setOrientation] = useState<BrowseOrientation>("straight");
  const [filter, setFilter] = useState("");
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
        <ul className="max-h-[60vh] overflow-y-auto rounded-md border border-[var(--color-border)] divide-y divide-[var(--color-border)]">
          {categories.map((cat) => (
            <li key={`${cat.orientation}-${cat.slug}-${cat.categoryId ?? ""}`}>
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
            </li>
          ))}
        </ul>
      </CardContent>
    </Card>
  );
}
