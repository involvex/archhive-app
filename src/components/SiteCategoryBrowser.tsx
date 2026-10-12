import { useCallback, useEffect, useMemo, useState } from "react";
import { Link } from "@tanstack/react-router";
import { RefreshCw, Search } from "lucide-react";
import { api } from "@/lib/api/client";
import type { SiteCategory } from "@/lib/types";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { ErrorState } from "@/components/ErrorState";

/** Per-session cache so returning to a site hub doesn't refetch. */
const cache = new Map<string, SiteCategory[]>();

function formatCount(n?: number): string {
  if (n == null) return "";
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${Math.round(n / 1_000)}K`;
  return String(n);
}

/** Category / tag index for a site, fetched from the backend adapter. */
export function SiteCategoryBrowser({ siteId, siteName }: { siteId: string; siteName: string }) {
  const [categories, setCategories] = useState<SiteCategory[] | null>(
    () => cache.get(siteId) ?? null,
  );
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [filter, setFilter] = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const list = await api.listSiteCategories(siteId);
      cache.set(siteId, list);
      setCategories(list);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Could not load categories");
    } finally {
      setLoading(false);
    }
  }, [siteId]);

  useEffect(() => {
    if (cache.has(siteId)) return;
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void load();
  }, [siteId, load]);

  const visible = useMemo(() => {
    const q = filter.trim().toLowerCase();
    const list = categories ?? [];
    return q ? list.filter((c) => c.name.toLowerCase().includes(q)) : list;
  }, [categories, filter]);

  const kindLabel = categories?.every((c) => c.kind === "tag") ? "Tags" : "Categories";

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
          onClick={() => void load()}
          disabled={loading}
          title="Refresh"
        >
          <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
        </Button>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="relative max-w-sm">
          <Search className="absolute left-2 top-2.5 h-4 w-4 text-[var(--color-muted-foreground)]" />
          <Input
            className="pl-8"
            placeholder={`Filter ${kindLabel.toLowerCase()}…`}
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
          />
        </div>
        {error && <ErrorState message={error} onRetry={() => void load()} />}
        {loading && !categories && (
          <p className="text-xs text-[var(--color-muted-foreground)]">Loading…</p>
        )}
        {categories && categories.length === 0 && !loading && !error && (
          <p className="text-xs text-[var(--color-muted-foreground)]">
            No categories returned. The site may require cookies — log in from Settings → Cookies.
          </p>
        )}
        <div className="flex max-h-[60vh] flex-wrap gap-1.5 overflow-y-auto">
          {visible.map((c) => (
            <Link
              key={`${c.kind}:${c.slug}`}
              to="/browse/$site/$kind/$slug"
              params={{ site: siteId, kind: c.kind, slug: c.slug }}
              className="inline-flex items-center gap-1.5 rounded-full bg-[var(--color-secondary)] px-3 py-1 text-xs transition hover:bg-[var(--color-muted)]"
            >
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
                <span className="text-[10px] text-[var(--color-muted-foreground)]">
                  {formatCount(c.count)}
                </span>
              )}
            </Link>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
