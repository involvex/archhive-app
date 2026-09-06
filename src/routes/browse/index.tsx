import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "@/lib/api/client";
import { getCapabilities } from "@/lib/runtime";
import { getPluginBrowseSites } from "@/lib/plugins/loader";
import { mergeSiteLists, PORNHUB_FEED_SLUG } from "@/lib/sites/catalog";
import { useSettingsStore } from "@/lib/stores/settings";
import type {
  BrowseOrientation,
  MediaItem,
  PornhubCategoryEntry,
  SavedSearch,
  SiteInfo,
} from "@/lib/types";
import { SceneCard } from "@/components/SceneCard";
import { Skeleton } from "@/components/ui/skeleton";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { ConnectionStatusChip } from "@/components/ConnectionStatusChip";
import {
  Globe,
  Link2,
  Search,
  Clock,
  Radio,
  ArrowRight,
  Tags,
  Heart,
  Newspaper,
  Flame,
  Bookmark,
  RefreshCw,
  Trash2,
} from "lucide-react";
import { isMobileDevice } from "@/lib/tauri";

const ORIENTATIONS: { value: BrowseOrientation; label: string }[] = [
  { value: "straight", label: "Straight" },
  { value: "gay", label: "Gay" },
  { value: "lesbian", label: "Lesbian" },
  { value: "transgender", label: "Trans" },
];

export const Route = createFileRoute("/browse/")({
  component: BrowsePage,
});

function useRecentSearches() {
  const [recent, setRecent] = useState<Array<{ site: string; kind: string; slug: string }>>(() => {
    try {
      const raw = localStorage.getItem("archhive_recent_searches");
      return raw ? JSON.parse(raw) : [];
    } catch {
      return [];
    }
  });

  const addRecent = useCallback((entry: { site: string; kind: string; slug: string }) => {
    setRecent((prev) => {
      const filtered = prev.filter((r) => r.slug !== entry.slug || r.site !== entry.site);
      const next = [entry, ...filtered].slice(0, 10);
      try {
        localStorage.setItem("archhive_recent_searches", JSON.stringify(next));
      } catch {
        /* ignore */
      }
      return next;
    });
  }, []);

  return { recent, addRecent };
}

function BrowsePage() {
  const caps = getCapabilities();
  const navigate = useNavigate();
  const { settings } = useSettingsStore();
  const isMobile = isMobileDevice();
  const { recent, addRecent } = useRecentSearches();
  const [sites, setSites] = useState<SiteInfo[]>(() => mergeSiteLists([], getPluginBrowseSites()));
  const sitesLoadedRef = useRef(false);
  const sitesRef = useRef(sites);

  useEffect(() => {
    sitesRef.current = sites;
  }, [sites]);

  const [selectedSite, setSelectedSite] = useState("auto");
  const [searchInput, setSearchInput] = useState("");
  const [urlInput, setUrlInput] = useState("");
  const [loadError, setLoadError] = useState("");
  const [urlError, setUrlError] = useState("");
  const [urlLoading, setUrlLoading] = useState(false);
  const needsRemoteSetup =
    caps.showBrowserBanner ||
    (settings.engine_mode === "remote_lan" && !settings.remote_host?.trim());

  useEffect(() => {
    if (needsRemoteSetup && caps.showBrowserBanner) return;
    void api
      .listSites()
      .then((apiSites) => {
        setSites(mergeSiteLists(apiSites, getPluginBrowseSites()));
        sitesLoadedRef.current = true;
      })
      .catch((e) => {
        setLoadError(e instanceof Error ? e.message : "Failed to load sites");
        setSites(mergeSiteLists([], getPluginBrowseSites()));
        sitesLoadedRef.current = true;
      });
  }, [needsRemoteSetup, caps.showBrowserBanner]);

  const handleSearch = useCallback(() => {
    let slug = searchInput.trim();
    if (!slug) return;

    let site = selectedSite;
    let kind: string = "search";

    if (slug.startsWith("#")) {
      slug = slug.slice(1).trim();
      if (!slug) return;
      kind = "tag";
    }

    if (site === "auto") {
      if (slug.startsWith("http")) {
        navigate({ to: "/browse/by-url", search: { url: slug } });
        return;
      }
      site = sites[0]?.id ?? "custom";
    }

    const targetSite = sites.find((s) => s.id === site);
    if (targetSite) {
      if (kind === "search" && !targetSite.supported_kinds.includes("search")) {
        kind = targetSite.supported_kinds[0];
      }
      addRecent({ site: targetSite.id, kind, slug });
      navigate({
        to: "/browse/$site/$kind/$slug",
        params: { site: targetSite.id, kind, slug },
      });
    }
  }, [searchInput, selectedSite, sites, navigate, addRecent]);

  const handlePasteUrl = useCallback(async () => {
    const url = urlInput.trim();
    if (!url) return;
    setUrlLoading(true);
    setUrlError("");
    try {
      await api.queueDownload(url);
      setUrlInput("");
    } catch (e) {
      setUrlError(e instanceof Error ? e.message : "Download failed");
    } finally {
      setUrlLoading(false);
    }
  }, [urlInput]);

  const handleSiteChip = useCallback((siteId: string) => {
    setSelectedSite(siteId);
  }, []);

  const [categories, setCategories] = useState<PornhubCategoryEntry[]>([]);
  const [catOrientation, setCatOrientation] = useState<BrowseOrientation>("straight");
  const [catLoading, setCatLoading] = useState(false);

  const [trending, setTrending] = useState<Record<string, MediaItem[]>>({});
  const [trendingLoading, setTrendingLoading] = useState(false);

  // #28 saved searches (watchlist).
  const [saved, setSaved] = useState<SavedSearch[]>([]);
  const [checkingId, setCheckingId] = useState<string | null>(null);

  const refreshSaved = useCallback(() => {
    void api
      .listSavedSearches()
      .then(setSaved)
      .catch(() => setSaved([]));
  }, []);

  useEffect(() => {
    refreshSaved();
  }, [refreshSaved]);

  async function handleCheck(id: string) {
    setCheckingId(id);
    try {
      await api.checkSavedSearch(id);
      await refreshSaved();
    } catch (e) {
      console.error(e);
    } finally {
      setCheckingId(null);
    }
  }

  async function handleDeleteSaved(id: string) {
    try {
      await api.deleteSavedSearch(id);
      setSaved((prev) => prev.filter((s) => s.id !== id));
    } catch (e) {
      console.error(e);
    }
  }

  useEffect(() => {
    if (selectedSite !== "pornhub") {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setCategories([]);
      return;
    }
    setCatLoading(true);
    void api
      .listPornhubCategories(catOrientation)
      .then(setCategories)
      .catch(() => setCategories([]))
      .finally(() => setCatLoading(false));
  }, [selectedSite, catOrientation]);

  useEffect(() => {
    if (needsRemoteSetup && caps.showBrowserBanner) return;
    if (!sitesLoadedRef.current) return;
    setTrendingLoading(true);
    const enabled = new Set(settings.trending_sites ?? []);
    const available = sitesRef.current.filter((s) => enabled.has(s.id));
    if (available.length === 0) {
      setTrending({});
      setTrendingLoading(false);
      return;
    }
    void Promise.allSettled(
      available.map((s) =>
        api.browse(s.id, "search", "trending", 1).then((page) => ({
          id: s.id,
          items: page.items.slice(0, 10),
        })),
      ),
    ).then((results) => {
      const next: Record<string, MediaItem[]> = {};
      results.forEach((r) => {
        if (r.status === "fulfilled") {
          next[r.value.id] = r.value.items;
        }
      });
      setTrending(next);
      setTrendingLoading(false);
    });
  }, [needsRemoteSetup, caps.showBrowserBanner, settings.trending_sites]);

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">Browse</h2>
        <p className="text-sm text-[var(--color-muted-foreground)]">
          Search across sites or paste a URL
        </p>
      </div>

      {(isMobile || caps.showBrowserBanner) && <ConnectionStatusChip />}

      {caps.showBrowserBanner && !needsRemoteSetup && (
        <Card className="border-[var(--color-border)]">
          <CardContent className="p-3 text-xs text-[var(--color-muted-foreground)]">
            Browser mode — API calls go to your configured Remote LAN host.
          </CardContent>
        </Card>
      )}

      {needsRemoteSetup && (
        <Card className="border-[var(--color-primary)]">
          <CardContent className="space-y-2 p-4 text-sm">
            <p>
              Connect to your desktop ArcHive: open <strong>Settings → Engine → Remote LAN</strong>.
            </p>
            <p className="text-xs text-[var(--color-muted-foreground)]">
              Host: <code>http://192.168.178.69:8787</code> — enable LAN on the PC app first.
            </p>
          </CardContent>
        </Card>
      )}

      {loadError && (
        <Card className="border-yellow-600/50 bg-yellow-950/30">
          <CardContent className="p-3 text-sm text-yellow-200">
            {loadError} — showing offline site list.
          </CardContent>
        </Card>
      )}

      <Card>
        <CardContent className="p-4 space-y-3">
          <div className="flex flex-col sm:flex-row gap-2">
            <div className="flex-1 flex gap-2">
              <select
                className="h-10 shrink-0 rounded-md border border-[var(--color-border)] bg-[var(--color-background)] px-3 text-sm"
                value={selectedSite}
                onChange={(e) => setSelectedSite(e.target.value)}
              >
                <option value="auto">Auto-detect</option>
                {sites.map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.display_name}
                  </option>
                ))}
              </select>
              <Input
                placeholder='Search (e.g. "model name", "#tag", or paste URL)'
                value={searchInput}
                onChange={(e) => setSearchInput(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleSearch()}
                className="flex-1"
              />
            </div>
            <Button onClick={handleSearch} disabled={!searchInput.trim()} className="h-10 shrink-0">
              <Search className="h-4 w-4 sm:mr-2" />
              <span className="hidden sm:inline">Browse</span>
            </Button>
          </div>

          <div className="flex gap-1.5 overflow-x-auto pb-1 -mx-1 px-1 snap-x snap-mandatory scrollbar-hide">
            {sites.map((site) => (
              <button
                key={site.id}
                type="button"
                onClick={() => handleSiteChip(site.id)}
                className={`inline-flex items-center gap-1 rounded-full px-3 py-1.5 text-xs font-medium transition shrink-0 snap-start ${
                  selectedSite === site.id
                    ? "bg-[var(--color-primary)] text-[var(--color-primary-foreground)]"
                    : "bg-[var(--color-secondary)] hover:bg-[var(--color-muted)]"
                }`}
              >
                <Globe className="h-3 w-3" />
                {site.display_name}
                {site.requires_cookies && <span className="text-[10px] opacity-70">(cookies)</span>}
              </button>
            ))}
          </div>
        </CardContent>
      </Card>

      <div className="flex flex-wrap gap-2">
        <Button
          size="sm"
          variant="secondary"
          onClick={() =>
            navigate({
              to: "/browse/$site/$kind/$slug",
              params: { site: "pornhub", kind: "category", slug: "lesbian" },
            })
          }
        >
          <Heart className="h-4 w-4 mr-1.5" />
          PornHub Lesbian
        </Button>
        <Button
          size="sm"
          variant="secondary"
          onClick={() =>
            navigate({
              to: "/browse/$site/$kind/$slug",
              params: { site: "pornhub", kind: "search", slug: PORNHUB_FEED_SLUG },
            })
          }
        >
          <Newspaper className="h-4 w-4 mr-1.5" />
          News / Feed
        </Button>
      </div>

      <Card>
        <CardContent className="p-4 space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 text-sm font-medium">
              <Flame className="h-4 w-4 text-orange-500" />
              <span>Trending</span>
            </div>
          </div>
          {trendingLoading ? (
            <div className="flex gap-3 overflow-hidden">
              {["a", "b", "c", "d", "e", "f", "g", "h"].map((k) => (
                <div key={k} className="shrink-0 w-44">
                  <Skeleton className="aspect-video w-full rounded-lg" />
                  <Skeleton className="mt-2 h-4 w-3/4" />
                  <Skeleton className="mt-1.5 h-3 w-1/2" />
                </div>
              ))}
            </div>
          ) : (
            <div className="flex gap-3 overflow-x-auto pb-2 -mx-1 px-1 snap-x snap-mandatory scrollbar-hide">
              {Object.values(trending).map((items) =>
                items.map((item) => (
                  <div key={item.id} className="shrink-0 w-44 snap-start">
                    <SceneCard
                      item={item}
                      listView={false}
                      onClick={() => {
                        navigate({
                          to: "/browse/$site/$kind/$slug",
                          params: {
                            site: item.site_id,
                            kind: "search",
                            slug: encodeURIComponent(item.title),
                          },
                        });
                      }}
                    />
                  </div>
                )),
              )}
              {Object.values(trending).every((items) => items.length === 0) && (
                <p className="text-xs text-[var(--color-muted-foreground)]">
                  No trending content available yet.
                </p>
              )}
            </div>
          )}
        </CardContent>
      </Card>

      {selectedSite === "pornhub" && (
        <Card>
          <CardContent className="p-4 space-y-3">
            <div className="flex items-center gap-2 text-sm font-medium">
              <Tags className="h-4 w-4" />
              <span>PornHub Categories</span>
            </div>
            <div className="flex flex-wrap gap-1.5">
              {ORIENTATIONS.map((o) => (
                <button
                  key={o.value}
                  type="button"
                  onClick={() => setCatOrientation(o.value)}
                  className={`inline-flex items-center rounded-full px-3 py-1 text-xs font-medium transition ${
                    catOrientation === o.value
                      ? "bg-[var(--color-primary)] text-[var(--color-primary-foreground)]"
                      : "bg-[var(--color-secondary)] hover:bg-[var(--color-muted)]"
                  }`}
                >
                  {o.label}
                </button>
              ))}
            </div>
            {catLoading ? (
              <p className="text-xs text-[var(--color-muted-foreground)]">Loading categories…</p>
            ) : (
              <div className="flex flex-wrap gap-1.5 max-h-60 overflow-y-auto">
                {categories.map((cat) => (
                  <button
                    key={`${cat.orientation}:${cat.slug}`}
                    type="button"
                    onClick={() => {
                      navigate({
                        to: "/browse/$site/$kind/$slug",
                        params: {
                          site: "pornhub",
                          kind: "category",
                          slug: cat.slug,
                        },
                      });
                    }}
                    className="inline-flex items-center gap-1 rounded-full bg-[var(--color-secondary)] px-2.5 py-1 text-xs hover:bg-[var(--color-primary)]/30 hover:text-[var(--color-primary)] transition-colors"
                  >
                    {cat.name}
                    {cat.video_count !== undefined && (
                      <span className="text-[10px] opacity-60 tabular-nums">
                        {cat.video_count.toLocaleString()}
                      </span>
                    )}
                  </button>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      )}

      <Card>
        <CardContent className="p-4 space-y-3">
          <div className="flex items-center gap-2 text-sm font-medium">
            <Link2 className="h-4 w-4" />
            Quick paste URL
          </div>
          <div className="flex flex-col sm:flex-row gap-2">
            <Input
              placeholder="https://..."
              value={urlInput}
              onChange={(e) => setUrlInput(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && void handlePasteUrl()}
            />
            <Button
              onClick={() => void handlePasteUrl()}
              disabled={urlLoading || !urlInput.trim()}
              className="shrink-0"
            >
              Download
            </Button>
            <Button
              variant="outline"
              onClick={() =>
                urlInput.trim() &&
                navigate({ to: "/browse/by-url", search: { url: urlInput.trim() } })
              }
              disabled={!urlInput.trim()}
              className="shrink-0"
            >
              Browse URL
            </Button>
          </div>
          {urlError && <p className="text-xs text-red-400">{urlError}</p>}
        </CardContent>
      </Card>

      {recent.length > 0 && (
        <Card>
          <CardContent className="p-4 space-y-3">
            <div className="flex items-center gap-2 text-sm font-medium">
              <Clock className="h-4 w-4" />
              Recent searches
            </div>
            <div className="grid gap-1">
              {recent.map((r) => (
                <button
                  key={`${r.site}-${r.kind}-${r.slug}`}
                  type="button"
                  className="flex items-center justify-between rounded-md px-3 py-2 text-sm hover:bg-[var(--color-muted)] transition text-left"
                  onClick={() => {
                    setSelectedSite(r.site);
                    setSearchInput(r.slug);
                    navigate({
                      to: "/browse/$site/$kind/$slug",
                      params: { site: r.site, kind: r.kind, slug: encodeURIComponent(r.slug) },
                    });
                  }}
                >
                  <span className="flex items-center gap-2">
                    <span className="text-xs text-[var(--color-muted-foreground)] capitalize">
                      {r.site}
                    </span>
                    <span className="text-xs text-[var(--color-muted-foreground)]">{r.kind}:</span>
                    <span>{r.slug}</span>
                  </span>
                  <ArrowRight className="h-3.5 w-3.5 text-[var(--color-muted-foreground)]" />
                </button>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardContent className="p-4 space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 text-sm font-medium">
              <Bookmark className="h-4 w-4" />
              Saved searches
            </div>
            {saved.length > 0 && (
              <span className="text-xs text-[var(--color-muted-foreground)]">
                {saved.reduce((sum, s) => sum + s.new_count, 0)} new matches
              </span>
            )}
          </div>
          {saved.length === 0 ? (
            <p className="text-xs text-[var(--color-muted-foreground)]">
              No saved searches yet. Open any tag, model, channel, or search page and press Save to
              watch it for new content.
            </p>
          ) : (
            <div className="grid gap-1">
              {saved.map((s) => (
                <div
                  key={s.id}
                  className="flex items-center justify-between gap-2 rounded-md px-3 py-2 text-sm hover:bg-[var(--color-muted)] transition"
                >
                  <button
                    type="button"
                    className="flex min-w-0 flex-1 items-center gap-2 text-left"
                    onClick={() => {
                      navigate({
                        to: "/browse/$site/$kind/$slug",
                        params: { site: s.site_id, kind: s.kind, slug: encodeURIComponent(s.slug) },
                      });
                    }}
                  >
                    <span className="min-w-0 flex-1 truncate">
                      <span className="font-medium">{s.name}</span>{" "}
                      <span className="text-xs text-[var(--color-muted-foreground)]">
                        {s.last_checked_at
                          ? `· checked ${new Date(s.last_checked_at).toLocaleString()}`
                          : "· never checked"}
                      </span>
                    </span>
                    {s.new_count > 0 && (
                      <span className="shrink-0 rounded-full bg-[var(--color-primary)] px-2 py-0.5 text-[11px] font-semibold text-[var(--color-primary-foreground)] tabular-nums">
                        {s.new_count} new
                      </span>
                    )}
                  </button>
                  <div className="flex shrink-0 items-center gap-1">
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => void handleCheck(s.id)}
                      disabled={checkingId === s.id}
                      title="Check for new matches now"
                    >
                      <RefreshCw
                        className={`h-3.5 w-3.5 ${checkingId === s.id ? "animate-spin" : ""}`}
                      />
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => void handleDeleteSaved(s.id)}
                      title="Delete saved search"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      <div className="flex items-center gap-2 text-sm font-medium">
        <Radio className="h-4 w-4 text-red-500" />
        <span>Live Streams</span>
      </div>
      <Card>
        <CardContent className="p-4">
          <p className="text-sm text-[var(--color-muted-foreground)] mb-3">
            Browse live streams from Chaturbate and Stripchat.
          </p>
          <Button asChild variant="outline">
            <a
              href="/live"
              onClick={(e) => {
                e.preventDefault();
                navigate({ to: "/live" });
              }}
            >
              Open Live Streams
            </a>
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
