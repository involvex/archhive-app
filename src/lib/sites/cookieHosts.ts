/**
 * Per-domain cookie entries for sites without their own adapter: xPomelo
 * sources (SpankBang, FapCat, …) and any other site added by domain. The
 * key rule mirrors `sites::cookie_keys::host_key` in Rust:
 * `https://www.SpankBang.com/…` → `host-spankbang-com`.
 */

export interface CookieHost {
  name: string;
  domain: string;
  /** Sells videos (trailers only without a membership). */
  paysite?: boolean;
}

/** xPomelo sources that ArcHive has no dedicated adapter for. */
export const XPOMELO_COOKIE_HOSTS: CookieHost[] = [
  { name: "SpankBang", domain: "spankbang.com" },
  { name: "Noodlemagazine", domain: "noodlemagazine.com" },
  { name: "RedTube", domain: "redtube.com" },
  { name: "PornZog", domain: "pornzog.com" },
  { name: "XGroovy", domain: "xgroovy.com" },
  { name: "4KPorn", domain: "4kporn.xxx" },
  { name: "TNAFlix", domain: "tnaflix.com" },
  { name: "Love4Porn", domain: "love4porn.com" },
  { name: "PornOne", domain: "pornone.com" },
  { name: "Hoes", domain: "hoes.tube" },
  { name: "HDtube", domain: "hdtube.porn" },
  { name: "CrazyPorn", domain: "crazyporn.xxx" },
  { name: "Its.porn", domain: "its.porn" },
  { name: "FapCat", domain: "fapcat.com" },
  { name: "YouJizz", domain: "youjizz.com" },
  { name: "Tube8", domain: "tube8.com" },
  { name: "SxyPrn", domain: "sxyprn.com" },
  { name: "XXXBP", domain: "xxxbp.tv" },
  { name: "EroThots", domain: "erothots1.com" },
  { name: "HQPorner", domain: "hqporner.com" },
  { name: "PornHat", domain: "pornhat.com" },
  { name: "SexVid", domain: "sexvid.pro" },
  { name: "TXXX", domain: "txxx.com" },
  { name: "DrTuber", domain: "drtuber.com" },
  { name: "VipTube", domain: "viptube.com" },
  { name: "inXXX", domain: "inxxx.com" },
  { name: "AnySex", domain: "anysex.com" },
  { name: "Analdin", domain: "analdin.com" },
  { name: "Beeg", domain: "beeg.com" },
  { name: "Xozilla", domain: "xozilla.com" },
  { name: "Nuvid", domain: "nuvid.com" },
  { name: "Wankoz", domain: "wankoz.com" },
  { name: "FapHouse", domain: "faphouse.com", paysite: true },
  { name: "Clips4Sale", domain: "clips4sale.com", paysite: true },
  { name: "Brazzers", domain: "brazzers.com", paysite: true },
  { name: "RealityKings", domain: "realitykings.com", paysite: true },
  { name: "PureTaboo", domain: "puretaboo.com", paysite: true },
  { name: "AnalVids", domain: "analvids.com", paysite: true },
];

/** `https://www.Site.com/x` or `site.com` → `host-site-com` (null if not a domain). */
export function hostCookieKey(urlOrDomain: string): string | null {
  let host = urlOrDomain.trim();
  if (host.includes("://")) {
    try {
      host = new URL(host).hostname;
    } catch {
      return null;
    }
  } else {
    host = host.replace(/\/.*$/, "");
  }
  host = host
    .toLowerCase()
    .replace(/^\.+/, "")
    .replace(/^www\./, "");
  if (!host.includes(".") || /\s/.test(host)) return null;
  return `host-${host.replace(/[^a-z0-9]/g, "-")}`;
}

export function isHostCookieKey(key: string): boolean {
  return key.startsWith("host-");
}

/** Does a cookie's domain belong to per-domain entry `key` (subdomains too)? */
export function cookieDomainMatchesHostKey(cookieDomain: string, key: string): boolean {
  const labels = cookieDomain.toLowerCase().replace(/^\.+/, "").split(".");
  for (let i = 0; i + 2 <= labels.length; i++) {
    if (hostCookieKey(labels.slice(i).join(".")) === key) return true;
  }
  return false;
}

/** Display name for a vault key: known source name, else the key itself. */
export function cookieHostLabel(key: string, extra: CookieHost[] = []): string {
  const hit = [...XPOMELO_COOKIE_HOSTS, ...extra].find((h) => hostCookieKey(h.domain) === key);
  return hit ? `${hit.name} (${hit.domain})` : key;
}
