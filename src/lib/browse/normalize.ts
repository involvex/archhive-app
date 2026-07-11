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
  } catch {
    /* keep defaults */
  }

  return { kind: "video", slug: trimmed };
}
