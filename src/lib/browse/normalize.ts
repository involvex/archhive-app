import type { BrowseKind } from "../types";

/** Normalize user input for browse API calls (URLs vs slugs). */
export function normalizeBrowseInput(
  site: string,
  kind: BrowseKind,
  raw: string,
): { kind: BrowseKind; slug: string } {
  const trimmed = raw.trim();
  if (!trimmed.startsWith("http://") && !trimmed.startsWith("https://")) {
    return { kind, slug: trimmed };
  }

  try {
    const parsed = new URL(trimmed);
    if (site === "reddit") {
      if (parsed.pathname.includes("/comments/") || parsed.hostname.includes("v.redd.it")) {
        return { kind: "video", slug: trimmed };
      }
      const sub = parsed.pathname.match(/^\/r\/([^/]+)/)?.[1];
      if (sub) return { kind: "channel", slug: sub };
    }
    if (site === "thothub") {
      const tag = parsed.pathname.match(/\/tags\/([^/]+)/)?.[1];
      if (tag) return { kind: "tag", slug: decodeURIComponent(tag) };
      const videos = parsed.pathname.match(/\/videos\/(\d+)/)?.[1];
      if (videos) return { kind: "video", slug: trimmed };
      const model = parsed.pathname.match(/\/model\/([^/]+)/)?.[1];
      if (model) return { kind: "tag", slug: decodeURIComponent(model) };
    }
    if (site === "youporn") {
      const category = parsed.pathname.match(/\/category\/([^/]+)/)?.[1];
      if (category) return { kind: "category", slug: decodeURIComponent(category) };
      if (parsed.pathname.includes("/categories")) {
        return { kind: "category", slug: "categories" };
      }
      const pornstar = parsed.pathname.match(/\/pornstar\/([^/]+)/)?.[1];
      if (pornstar) return { kind: "model", slug: decodeURIComponent(pornstar) };
      const watch = parsed.pathname.match(/\/watch\/(\d+)/)?.[1];
      if (watch) return { kind: "video", slug: trimmed };
    }
    if (site === "xnxx") {
      const search = parsed.pathname.match(/\/search\/([^/]+)/)?.[1];
      if (search) return { kind: "search", slug: decodeURIComponent(search) };
      if (parsed.pathname.includes("/video-")) {
        return { kind: "video", slug: trimmed };
      }
    }
    if (site === "instagram") {
      if (parsed.pathname.includes("/p/") || parsed.pathname.includes("/reel/")) {
        return { kind: "video", slug: trimmed };
      }
      const user = parsed.pathname.match(/^\/([^/]+)\/?$/)?.[1];
      if (user && !["p", "reel", "reels", "stories", "explore", "accounts"].includes(user)) {
        return { kind: "channel", slug: user };
      }
    }
  } catch {
    /* keep defaults */
  }

  return { kind: "video", slug: trimmed };
}
