import { createFileRoute, Link } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { api } from "@/lib/api/client";
import type { SiteInfo } from "@/lib/types";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Globe, Search } from "lucide-react";

export const Route = createFileRoute("/browse/")({
  component: BrowsePage,
});

function BrowsePage() {
  const [sites, setSites] = useState<SiteInfo[]>([]);
  const [url, setUrl] = useState("");
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    void api.listSites().then(setSites).catch(console.error);
  }, []);

  async function handlePasteDownload() {
    if (!url.trim()) return;
    setLoading(true);
    try {
      await api.queueDownload(url.trim());
      setUrl("");
    } catch (e) {
      console.error(e);
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
                  <Link
                    key={kind}
                    to="/browse/$site/$kind/$slug"
                    params={{ site: site.id, kind, slug: "example" }}
                    className="rounded-full bg-[var(--color-secondary)] px-2 py-0.5 text-xs hover:bg-[var(--color-primary)] hover:text-[var(--color-primary-foreground)]"
                  >
                    {kind}
                  </Link>
                ))}
              </div>
              {site.requires_cookies && (
                <span className="text-xs text-yellow-400">Requires cookies</span>
              )}
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}
