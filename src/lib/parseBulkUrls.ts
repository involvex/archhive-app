export interface ParsedBulkUrls {
  videos: string[];
  browse: string[];
  other: string[];
}

const URL_RE = /https?:\/\/[^\s<>"')\]]+/gi;

/** Extract unique HTTP(S) URLs from numbered lists, plain text, or newline-separated paste. */
export function parseBulkUrls(text: string): string[] {
  const matches = text.match(URL_RE) ?? [];
  const cleaned = matches.map((u) => u.replace(/[.,;]+$/g, "").trim()).filter(Boolean);
  return [...new Set(cleaned)];
}

/** Channel, search, or playlist listing pages (expand via yt-dlp flat-playlist). */
export function isBrowseUrl(url: string): boolean {
  try {
    const u = new URL(url);
    const path = u.pathname.toLowerCase();
    if (path.includes("/search") || path.includes("/channel/") || path.includes("/channels/")) {
      return true;
    }
    if (u.searchParams.has("query")) return true;
    return false;
  } catch {
    return false;
  }
}

/** True when URL looks like a single video/watch page. */
export function isLikelyVideoUrl(url: string): boolean {
  if (isBrowseUrl(url)) return false;
  try {
    const u = new URL(url);
    const path = u.pathname.toLowerCase();
    const host = u.hostname.toLowerCase();
    if (path.includes("/watch") || path.includes("/video")) return true;
    if (host.includes("youtu.be")) return true;
    if (host.includes("youtube.com") && u.searchParams.has("v")) return true;
    if (host.includes("redgifs.com") && path.length > 1) return true;
    return false;
  } catch {
    return false;
  }
}

export function classifyBulkUrls(text: string, importAll = false): ParsedBulkUrls {
  const all = parseBulkUrls(text);
  const videos: string[] = [];
  const browse: string[] = [];
  const other: string[] = [];

  for (const url of all) {
    if (isBrowseUrl(url)) {
      browse.push(url);
    } else if (isLikelyVideoUrl(url) || importAll) {
      videos.push(url);
    } else {
      other.push(url);
    }
  }

  return { videos, browse, other };
}
