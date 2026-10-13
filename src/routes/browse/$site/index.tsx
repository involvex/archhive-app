import { createFileRoute, Link } from "@tanstack/react-router";
import { mergeSiteLists, siteHasCategories, SITE_CATALOG } from "@/lib/sites/catalog";
import { useSiteInfo } from "@/lib/sites/useSiteInfo";
import { PornhubCategoryBrowser } from "@/components/PornhubCategoryBrowser";
import { SiteCategoryBrowser } from "@/components/SiteCategoryBrowser";
import { Card, CardContent } from "@/components/ui/card";
import { ArrowLeft } from "lucide-react";
import { SiteAvatar } from "@/components/SitePicker";
import { Button } from "@/components/ui/button";
import { CookieLoginButton } from "@/components/CookieLoginButton";
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
  livestream: "Livestream",
};

function SiteHubPage() {
  const { site: siteId } = Route.useParams();
  const liveSite = useSiteInfo(siteId);
  const site =
    liveSite ??
    mergeSiteLists([]).find((s) => s.id === siteId) ??
    SITE_CATALOG.find((s) => s.id === siteId);

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
    <div className="space-y-5">
      <div className="flex items-center gap-2">
        <Link
          to="/browse"
          aria-label="All sites"
          className="-ml-1.5 shrink-0 rounded-full p-1.5 text-[var(--color-muted-foreground)] hover:bg-[var(--color-muted)] hover:text-[var(--color-foreground)]"
        >
          <ArrowLeft className="h-5 w-5" />
        </Link>
        <SiteAvatar site={site} size="sm" />
        <div className="min-w-0 flex-1">
          <h2 className="truncate text-lg font-bold leading-tight md:text-2xl">
            {site.display_name}
          </h2>
          <p className="hidden truncate text-xs text-[var(--color-muted-foreground)] md:block">
            {site.base_url}
          </p>
        </div>
      </div>

      {site.id === "pornhub" && <PornhubCategoryBrowser />}

      {site.id !== "pornhub" && siteHasCategories(site) && (
        <SiteCategoryBrowser
          siteId={site.id}
          siteName={site.display_name}
          combineTags={site.browse_options?.combine_tags ?? 0}
          groups={site.browse_options?.category_groups ?? []}
          options={site.browse_options}
          canExclude={Boolean(
            site.browse_options?.include_exclude &&
            (site.browse_options.include_exclude_kinds?.length
              ? site.browse_options.include_exclude_kinds
              : site.browse_options.filter_kinds
            ).includes("category"),
          )}
        />
      )}

      {site.supported_kinds.length > 0 && (
        <div className="space-y-2">
          <p className="text-xs font-medium tracking-wide text-[var(--color-muted-foreground)] uppercase">
            Browse by
          </p>
          <div className="flex flex-wrap gap-1.5">
            {(site.id === "pornhub" ? advancedKinds : site.supported_kinds).map((kind) => (
              <Link
                key={kind}
                to="/browse/$site/$kind/$slug"
                params={{ site: site.id, kind, slug: "example" }}
                className="rounded-full bg-[var(--color-secondary)] px-3.5 py-1.5 text-sm hover:bg-[var(--color-muted)]"
              >
                {KIND_LABELS[kind]}
              </Link>
            ))}
          </div>
        </div>
      )}

      {site.requires_cookies && (
        <Card className="border-yellow-600/50 bg-yellow-950/30">
          <CardContent className="p-4 text-sm text-yellow-200">
            <strong>{site.display_name}</strong> requires cookies for browse and download. Log in
            below, or import them in <Link to="/settings">Settings → Cookies</Link>.
            <CookieLoginButton className="mt-3" siteId={site.id} siteName={site.display_name} />
          </CardContent>
        </Card>
      )}
    </div>
  );
}
