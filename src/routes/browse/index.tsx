import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import { getCapabilities } from "@/lib/runtime";
import { getPluginBrowseSites } from "@/lib/plugins/loader";
import { mergeSiteLists, PORNHUB_FEED_SLUG } from "@/lib/sites/catalog";
import { useSettingsStore } from "@/lib/stores/settings";
import type { BrowseOrientation, PornhubCategoryEntry, SiteInfo } from "@/lib/types";
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
      .then((apiSites) => setSites(mergeSiteLists(apiSites, getPluginBrowseSites())))
      .catch((e) => {
        setLoadError(e instanceof Error ? e.message : "Failed to load sites");
        setSites(mergeSiteLists([], getPluginBrowseSites()));
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

  useEffect(() => {
    if (selectedSite !== "pornhub") {
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
