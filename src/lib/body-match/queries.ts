/** BodyMatch query builder (V1, frontend-only).
 *
 * Generates external search URLs from cup size + hair color. All queries are
 * English (site search indexes are English-first). No backend calls.
 */

/** Cup sizes selectable in the UI. */
export type CupId = "A" | "B" | "C" | "D" | "DD" | "F" | "G" | "H+" | "big-tits" | "huge-tits";

/** Hair colors selectable in the UI. `any` means no hair filter. */
export type HairId = "blonde" | "brunette" | "black" | "red" | "auburn" | "any";

export interface BodyMatchInput {
  cup: CupId;
  hair: HairId;
}

export interface BodyMatchLink {
  /** Stable site key, e.g. "pornhub". */
  site: string;
  /** Display name, e.g. "Pornhub". */
  label: string;
  /** Human-readable query shown on the card. */
  query: string;
  /** Fully encoded external URL. */
  url: string;
}

export const CUP_OPTIONS: Array<{ value: CupId; label: string }> = [
  { value: "A", label: "A" },
  { value: "B", label: "B" },
  { value: "C", label: "C" },
  { value: "D", label: "D" },
  { value: "DD", label: "DD / E" },
  { value: "F", label: "F" },
  { value: "G", label: "G" },
  { value: "H+", label: "H+" },
  { value: "big-tits", label: "Big Tits" },
  { value: "huge-tits", label: "Huge Tits" },
];

export const HAIR_OPTIONS: Array<{ value: HairId; label: string }> = [
  { value: "blonde", label: "Blonde" },
  { value: "brunette", label: "Brunette" },
  { value: "black", label: "Black" },
  { value: "red", label: "Red" },
  { value: "auburn", label: "Auburn" },
  { value: "any", label: "Any" },
];

const CUP_PHRASE: Record<CupId, string> = {
  A: "A cup",
  B: "B cup",
  C: "C cup",
  D: "D cup",
  DD: "DD cup",
  F: "F cup",
  G: "G cup",
  "H+": "H cup",
  "big-tits": "big tits",
  "huge-tits": "huge tits",
};

const HAIR_PHRASE: Record<HairId, string> = {
  blonde: "blonde",
  brunette: "brunette",
  black: "black hair",
  red: "redhead",
  auburn: "auburn",
  any: "",
};

/** Google/IAFD needs an expanded OR-query for ambiguous cup letters. */
const CUP_GOOGLE_EXPANSION: Record<CupId, string> = {
  A: '"A cup"',
  B: '"B cup"',
  C: '"C cup"',
  D: '"D cup"',
  DD: '("DD cup" OR "E cup" OR 34DD OR 36DD)',
  F: '("F cup" OR "FF cup" OR 34F)',
  G: '("G cup" OR 34G)',
  "H+": '("H cup" OR "I cup" OR "J cup" OR "huge tits")',
  "big-tits": '"big tits"',
  "huge-tits": '"huge tits"',
};

function baseQuery({ cup, hair }: BodyMatchInput): string {
  const parts = [HAIR_PHRASE[hair], CUP_PHRASE[cup]].filter(Boolean);
  return parts.join(" ");
}

function googleQuery(input: BodyMatchInput): string {
  const hair = HAIR_PHRASE[input.hair];
  const cup = CUP_GOOGLE_EXPANSION[input.cup];
  return [hair, cup].filter(Boolean).join(" ");
}

/** Encode a query for `?search=` / `?k=` style params (spaces as %20). */
function enc(q: string): string {
  return encodeURIComponent(q);
}

/** Encode a query for SpankBang `/s/slug/` paths (spaces as `+`). */
function slug(q: string): string {
  return enc(q).replace(/%20/g, "+");
}

export function buildBodyMatchLinks(input: BodyMatchInput): BodyMatchLink[] {
  const q = baseQuery(input);
  const gq = googleQuery(input);
  return [
    {
      site: "pornhub",
      label: "Pornhub",
      query: q,
      url: `https://www.pornhub.com/video/search?search=${enc(q)}`,
    },
    {
      site: "xvideos",
      label: "XVIDEOS",
      query: q,
      url: `https://www.xvideos.com/?k=${enc(q)}`,
    },
    {
      site: "spankbang",
      label: "SpankBang",
      query: q,
      url: `https://spankbang.com/s/${slug(q)}/`,
    },
    {
      site: "boobpedia",
      label: "Boobpedia",
      query: q,
      url: `https://www.boobpedia.com/search?q=${enc(q)}`,
    },
    {
      site: "babepedia",
      label: "Babepedia",
      query: q,
      url: `https://www.babepedia.com/search?q=${enc(q)}`,
    },
    {
      site: "google-iafd",
      label: "Google + IAFD",
      query: `site:iafd.com ${gq}`,
      url: `https://www.google.com/search?q=${enc(`site:iafd.com ${gq}`)}`,
    },
  ];
}
