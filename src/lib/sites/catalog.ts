import type { BrowseKind, SiteInfo } from "../types";

/** Static fallback when LAN/API is unavailable — mirrors SiteRegistry in Rust. */
export const SITE_CATALOG: SiteInfo[] = [
  {
    id: "thothub",
    display_name: "ThotHub",
    base_url: "https://thethothub.com",
    supported_kinds: ["search", "tag"] as BrowseKind[],
    requires_cookies: false,
  },
  {
    id: "pornhub",
    display_name: "PornHub",
    base_url: "https://www.pornhub.com",
    supported_kinds: ["category", "search", "model", "channel", "tag"] as BrowseKind[],
    requires_cookies: true,
  },
  {
    id: "xhamster",
    display_name: "xHamster",
    base_url: "https://xhamster.com",
    supported_kinds: ["search", "channel", "tag"] as BrowseKind[],
    requires_cookies: false,
  },
  {
    id: "xvideos",
    display_name: "XVIDEOS",
    base_url: "https://www.xvideos.com",
    supported_kinds: ["search", "channel", "tag"] as BrowseKind[],
    requires_cookies: false,
  },
  {
    id: "reddit",
    display_name: "Reddit",
    base_url: "https://www.reddit.com",
    supported_kinds: ["search", "channel"] as BrowseKind[],
    requires_cookies: false,
  },
  {
    id: "redgifs",
    display_name: "RedGifs",
    base_url: "https://www.redgifs.com",
    supported_kinds: ["search", "tag"] as BrowseKind[],
    requires_cookies: false,
  },
  {
    id: "youtube",
    display_name: "YouTube",
    base_url: "https://www.youtube.com",
    supported_kinds: ["search", "channel"] as BrowseKind[],
    requires_cookies: false,
  },
  {
    id: "tiktok",
    display_name: "TikTok",
    base_url: "https://www.tiktok.com",
    supported_kinds: ["channel", "video"] as BrowseKind[],
    requires_cookies: false,
  },
  {
    id: "twitter",
    display_name: "Twitter / X",
    base_url: "https://x.com",
    supported_kinds: ["channel", "video"] as BrowseKind[],
    requires_cookies: false,
  },
  {
    id: "thisvid",
    display_name: "ThisVid",
    base_url: "https://thisvid.com",
    supported_kinds: ["search", "tag"] as BrowseKind[],
    requires_cookies: true,
  },
];

export function mergeSiteLists(
  apiSites: SiteInfo[],
  pluginSites: SiteInfo[] = [],
  fallback = SITE_CATALOG,
): SiteInfo[] {
  const base = apiSites.length === 0 ? fallback : apiSites;
  const byId = new Map(fallback.map((s) => [s.id, s]));
  for (const site of base) {
    byId.set(site.id, site);
  }
  for (const site of pluginSites) {
    byId.set(site.id, site);
  }
  return [...byId.values()];
}
