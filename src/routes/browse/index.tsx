import { createFileRoute, Link } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import { getCapabilities } from "@/lib/runtime";
import { mergeSiteLists, SITE_CATALOG } from "@/lib/sites/catalog";
import { useSettingsStore } from "@/lib/stores/settings";
import type { SiteInfo } from "@/lib/types";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Globe, Link2, Search } from "lucide-react";

export const Route = createFileRoute("/browse/")({
  component: BrowsePage,
});

function BrowsePage() {
  const caps = getCapabilities();
  const { settings } = useSettingsStore();
  const [sites, setSites] = useState<SiteInfo[]>(SITE_CATALOG);
  const [url, setUrl] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const needsRemoteSetup =
    caps.showBrowserBanner ||
    (settings.engine_mode === "remote_lan" && !settings.remote_host?.trim());

  useEffect(() => {
    if (needsRemoteSetup && caps.showBrowserBanner) {
      void queueMicrotask(() => setLoading(false));
      return;
    }
    void queueMicrotask(() => {
      setLoading(true);
      setError("");
    });
    void api
      .listSites()
      .then((apiSites) => setSites(mergeSiteLists(apiSites)))
      .catch((e) => {
        setError(e instanceof Error ? e.message : "Failed to load sites");
        setSites(SITE_CATALOG);
      })
      .finally(() => setLoading(false));
  }, [needsRemoteSetup, caps.showBrowserBanner, settings.remote_host]);

  async function handlePasteDownload() {
    if (!url.trim()) return;
    setLoading(true);
    try {
      await api.queueDownload(url.trim());
      setUrl("");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Download failed");
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">Browse</h2>
        <p className="text-sm text-[var(--color-muted-foreground)]">
          Pick a site or paste any supported URL
        </p>
      </div>

      {caps.showBrowserBanner && (
        <Card className="border-[var(--color-primary)]">
          <CardContent className="p-4 text-sm">
            You are viewing the dev UI in a browser. Configure{" "}
            <strong>Settings → Engine → Remote LAN</strong> with{" "}
            <code>http://&lt;pc-ip&gt;:8787</code> and your desktop LAN token, or use the desktop /
            Android app.
          </CardContent>
        </Card>
      )}

      {error && (
        <p className="rounded-md border border-yellow-600/50 bg-yellow-950/30 p-3 text-sm text-yellow-200">
          {error} — showing offline site list.
        </p>
      )}

      <Card className="border-[var(--color-primary)]">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <Link2 className="h-4 w-4" />
            Custom URL
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-2">
          <p className="text-sm text-[var(--color-muted-foreground)]">
            Paste any supported profile or playlist URL and browse via yt-dlp on the desktop host.
          </p>
          <Button asChild className="w-full sm:w-auto">
            <Link to="/browse/custom">Open Custom URL</Link>
          </Button>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <Search className="h-4 w-4" />
            Paste URL
          </CardTitle>
        </CardHeader>
        <CardContent className="flex gap-2">
          <Input
            placeholder="https://..."
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && void handlePasteDownload()}
          />
          <Button onClick={() => void handlePasteDownload()} disabled={loading}>
            Download
          </Button>
        </CardContent>
      </Card>

      <div>
        <h3 className="mb-3 text-sm font-medium text-[var(--color-muted-foreground)]">
          Pick a site
        </h3>
        {loading ? (
          <p className="text-sm text-[var(--color-muted-foreground)]">Loading sites…</p>
        ) : (
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {sites.map((site) => (
              <Card key={site.id} className="hover:border-[var(--color-primary)] transition">
                <CardHeader className="pb-2">
                  <CardTitle className="flex items-center gap-2 text-base">
                    <Globe className="h-4 w-4" />
                    {site.display_name}
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-2">
                  <p className="text-xs text-[var(--color-muted-foreground)]">{site.base_url}</p>
                  <div className="flex flex-wrap gap-1">
                    {site.supported_kinds.map((kind) => (
                      <span
                        key={kind}
                        className="rounded-full bg-[var(--color-secondary)] px-2 py-0.5 text-xs capitalize"
                      >
                        {kind}
                      </span>
                    ))}
                  </div>
                  {site.requires_cookies && (
                    <span className="text-xs text-yellow-400">Requires cookies</span>
                  )}
                  <Button asChild variant="outline" size="sm" className="w-full">
                    <Link to="/browse/$site" params={{ site: site.id }}>
                      Open site
                    </Link>
                  </Button>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
