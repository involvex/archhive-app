import { createFileRoute, Link } from "@tanstack/react-router";
import { mergeSiteLists, SITE_CATALOG } from "@/lib/sites/catalog";
import { PornhubCategoryBrowser } from "@/components/PornhubCategoryBrowser";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import type { BrowseKind } from "@/lib/types";

export const Route = createFileRoute("/browse/$site/")({
  component: SiteHubPage,
});

const KIND_LABELS: Record<BrowseKind, string> = {
  tag: "Tag",
  model: "Model",
  channel: "Channel",
  search: "Search",
  video: "Video",
  category: "Category",
};

function SiteHubPage() {
  const { site: siteId } = Route.useParams();
  const site =
    mergeSiteLists([]).find((s) => s.id === siteId) ?? SITE_CATALOG.find((s) => s.id === siteId);

  if (!site) {
    return (
      <div className="space-y-4">
        <h2 className="text-2xl font-bold">Unknown site</h2>
        <Button asChild variant="outline">
          <Link to="/browse">Back to Browse</Link>
        </Button>
      </div>
    );
  }

  const advancedKinds = site.supported_kinds.filter((k) => k !== "category");

  return (
    <div className="space-y-6">
      <div>
        <Button asChild variant="ghost" size="sm" className="mb-2">
          <Link to="/browse">← All sites</Link>
        </Button>
        <h2 className="text-2xl font-bold">{site.display_name}</h2>
        <p className="text-sm text-[var(--color-muted-foreground)]">{site.base_url}</p>
      </div>

      {site.id === "pornhub" && <PornhubCategoryBrowser />}

      {advancedKinds.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Advanced browse</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-wrap gap-2">
            {advancedKinds.map((kind) => (
              <Button key={kind} asChild variant="outline">
                <Link
                  to="/browse/$site/$kind/$slug"
                  params={{ site: site.id, kind, slug: "example" }}
                >
                  {KIND_LABELS[kind]}
                </Link>
              </Button>
            ))}
          </CardContent>
        </Card>
      )}

      {site.id !== "pornhub" && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Browse by</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-wrap gap-2">
            {site.supported_kinds.map((kind) => (
              <Button key={kind} asChild variant="outline">
                <Link
                  to="/browse/$site/$kind/$slug"
                  params={{ site: site.id, kind, slug: "example" }}
                >
                  {KIND_LABELS[kind]}
                </Link>
              </Button>
            ))}
          </CardContent>
        </Card>
      )}

      {site.requires_cookies && (
        <Card className="border-yellow-600/50 bg-yellow-950/30">
          <CardContent className="p-4 text-sm text-yellow-200">
            <strong>{site.display_name}</strong> requires cookies for browse and download. Import
            them in <Link to="/settings">Settings → Cookies</Link> before browsing.
          </CardContent>
        </Card>
      )}
    </div>
  );
}
