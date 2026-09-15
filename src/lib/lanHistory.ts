/**
 * Recent LAN hosts (Q35, #46). Remembers the last few successfully-used
 * `remote_host` values so reconnecting after a network change is one tap.
 * Only URLs are stored — never tokens.
 */

export interface LanHistoryEntry {
  url: string;
  /** mDNS name when the host came from discovery, if known. */
  name?: string;
  /** Whether a remote token was configured when this host was used. */
  hasToken: boolean;
  lastUsed: number;
}

const STORAGE_KEY = "archhive.lanHistory";
const MAX_ENTRIES = 3;

function normalizeUrl(url: string): string {
  return url.trim().replace(/\/+$/, "");
}

export function loadLanHistory(): LanHistoryEntry[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter(
        (e): e is LanHistoryEntry =>
          typeof e === "object" &&
          e !== null &&
          typeof (e as LanHistoryEntry).url === "string" &&
          Boolean((e as LanHistoryEntry).url.trim()),
      )
      .slice(0, MAX_ENTRIES);
  } catch {
    return [];
  }
}

/** Record a successfully-used host; returns the updated list. */
export function recordLanHost(
  url: string,
  opts: { name?: string; hasToken?: boolean } = {},
): LanHistoryEntry[] {
  const normalized = normalizeUrl(url);
  if (!normalized) return loadLanHistory();
  const next: LanHistoryEntry[] = [
    {
      url: normalized,
      name: opts.name?.trim() || undefined,
      hasToken: opts.hasToken ?? false,
      lastUsed: Date.now(),
    },
    ...loadLanHistory().filter((e) => normalizeUrl(e.url) !== normalized),
  ].slice(0, MAX_ENTRIES);
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
  } catch {
    /* private mode / quota — history just won't persist */
  }
  return next;
}

export function clearLanHistory(): void {
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch {
    /* ignore */
  }
}
