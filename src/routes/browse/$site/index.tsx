import { createFileRoute, Link } from "@tanstack/react-router";
import { mergeSiteLists, SITE_CATALOG } from "@/lib/sites/catalog";
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

  return (
    <div className="space-y-6">
      <div>
        <Button asChild variant="ghost" size="sm" className="mb-2">
          <Link to="/browse">← All sites</Link>
        </Button>
        <h2 className="text-2xl font-bold">{site.display_name}</h2>
        <p className="text-sm text-[var(--color-muted-foreground)]">{site.base_url}</p>
      </div>

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

      {site.requires_cookies && (
        <p className="text-sm text-yellow-400">
          This site may require cookies — import them in Settings → Cookies.
        </p>
      )}
    </div>
  );
}
