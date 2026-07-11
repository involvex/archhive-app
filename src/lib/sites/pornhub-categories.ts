import type { PornhubCategoryEntry } from "../types";

export type BrowseOrientation = "straight" | "gay" | "lesbian" | "transgender";

export interface PornhubCategory {
  name: string;
  slug: string;
  orientation: BrowseOrientation;
  /** Numeric PornHub `c` param when slug path is insufficient */
  categoryId?: number;
  videoCount?: number;
}

export const PORNHUB_ORIENTATIONS: { id: BrowseOrientation; label: string }[] = [
  { id: "straight", label: "Straight" },
  { id: "gay", label: "Gay" },
  { id: "lesbian", label: "Lesbian" },
  { id: "transgender", label: "Trans" },
];

/** Static catalog — straight and lesbian lists use separate entries per orientation. */
export const PORNHUB_CATEGORIES: PornhubCategory[] = [
  // Straight — popular
  { name: "Amateur", slug: "amateur", orientation: "straight", videoCount: 544892 },
  { name: "Anal", slug: "anal", orientation: "straight", videoCount: 147053 },
  { name: "Asian", slug: "asian", orientation: "straight", videoCount: 73235 },
  { name: "BBW", slug: "bbw", orientation: "straight", videoCount: 62670 },
  { name: "Big Ass", slug: "big-ass", orientation: "straight", videoCount: 342520 },
  { name: "Big Dick", slug: "big-dick", orientation: "straight", videoCount: 240262 },
  { name: "Big Tits", slug: "big-tits", orientation: "straight", videoCount: 313622 },
  { name: "Blonde", slug: "blonde", orientation: "straight", videoCount: 152471 },
  { name: "Blowjob", slug: "blowjob", orientation: "straight", videoCount: 230454 },
  { name: "Brunette", slug: "brunette", orientation: "straight", videoCount: 197055 },
  { name: "Creampie", slug: "creampie", orientation: "straight", videoCount: 121012 },
  { name: "Cumshot", slug: "cumshot", orientation: "straight", videoCount: 178204 },
  { name: "Ebony", slug: "ebony", orientation: "straight", videoCount: 48233 },
  { name: "German", slug: "german", orientation: "straight", videoCount: 17831 },
  { name: "Hardcore", slug: "hardcore", orientation: "straight", videoCount: 211484 },
  { name: "HD Porn", slug: "hd", orientation: "straight", videoCount: 1057035 },
  { name: "Interracial", slug: "interracial", orientation: "straight", videoCount: 40779 },
  { name: "Latina", slug: "latina", orientation: "straight", videoCount: 122793 },
  { name: "Lesbian", slug: "lesbian", orientation: "straight", categoryId: 27, videoCount: 42528 },
  { name: "MILF", slug: "milf", orientation: "straight", videoCount: 206901 },
  { name: "POV", slug: "pov", orientation: "straight", videoCount: 175227 },
  { name: "Pornstar", slug: "pornstar", orientation: "straight", videoCount: 193530 },
  { name: "Public", slug: "public", orientation: "straight", videoCount: 48840 },
  { name: "Russian", slug: "russian", orientation: "straight", videoCount: 35527 },
  { name: "Teen (18+)", slug: "teen", orientation: "straight", videoCount: 302003 },
  { name: "Threesome", slug: "threesome", orientation: "straight", videoCount: 42150 },
  {
    name: "Verified Amateurs",
    slug: "verified-amateurs",
    orientation: "straight",
    videoCount: 749827,
  },
  // Gay
  { name: "Gay Amateur", slug: "amateur", orientation: "gay", videoCount: 120000 },
  { name: "Gay Bareback", slug: "bareback", orientation: "gay", videoCount: 45000 },
  { name: "Gay Twink", slug: "twink", orientation: "gay", videoCount: 38000 },
  { name: "Gay Daddy", slug: "daddy", orientation: "gay", videoCount: 22000 },
  // Lesbian
  { name: "Lesbian", slug: "lesbian", orientation: "lesbian", categoryId: 27, videoCount: 42528 },
  { name: "Amateur", slug: "amateur", orientation: "lesbian", videoCount: 41174 },
  { name: "Anal", slug: "anal", orientation: "lesbian", videoCount: 9164 },
  { name: "Asian", slug: "asian", orientation: "lesbian", videoCount: 3911 },
  { name: "Big Ass", slug: "big-ass", orientation: "lesbian", videoCount: 37615 },
  { name: "Big Tits", slug: "big-tits", orientation: "lesbian", videoCount: 48524 },
  { name: "Blonde", slug: "blonde", orientation: "lesbian", videoCount: 31506 },
  { name: "Cunnilingus", slug: "cunnilingus", orientation: "lesbian", videoCount: 30705 },
  { name: "Ebony", slug: "ebony", orientation: "lesbian", videoCount: 9632 },
  { name: "Fingering", slug: "fingering", orientation: "lesbian", videoCount: 43312 },
  { name: "Latina", slug: "latina", orientation: "lesbian", videoCount: 17498 },
  { name: "MILF", slug: "milf", orientation: "lesbian", videoCount: 20653 },
  { name: "Orgasm", slug: "orgasm", orientation: "lesbian", videoCount: 30984 },
  { name: "POV", slug: "pov", orientation: "lesbian", videoCount: 5331 },
  { name: "Pussy Licking", slug: "pussy-licking", orientation: "lesbian", videoCount: 53286 },
  { name: "Romantic", slug: "romantic", orientation: "lesbian", videoCount: 10627 },
  { name: "Sapphic", slug: "sapphic", orientation: "lesbian", videoCount: 18855 },
  { name: "Scissoring", slug: "scissoring", orientation: "lesbian", videoCount: 19791 },
  { name: "Strap On", slug: "strap-on", orientation: "lesbian", videoCount: 14265 },
  { name: "Threesome", slug: "threesome", orientation: "lesbian", videoCount: 8355 },
  { name: "Toys", slug: "toys", orientation: "lesbian", videoCount: 24063 },
  { name: "Tribbing", slug: "tribbing", orientation: "lesbian", videoCount: 4650 },
  // Transgender
  { name: "Transgender", slug: "transgender", orientation: "transgender", videoCount: 31086 },
  { name: "Shemale", slug: "shemale", orientation: "transgender", videoCount: 28000 },
];

export function categoriesForOrientation(orientation: BrowseOrientation): PornhubCategory[] {
  return PORNHUB_CATEGORIES.filter((c) => c.orientation === orientation);
}

export function categoryBrowseSlug(cat: PornhubCategory): string {
  return cat.categoryId != null ? String(cat.categoryId) : cat.slug;
}

/** Merge live scraped counts/IDs into the static catalog. */
export function mergeCategoryCatalog(
  staticList: PornhubCategory[],
  live: PornhubCategoryEntry[],
): PornhubCategory[] {
  const liveByKey = new Map(live.map((c) => [`${c.orientation}:${c.slug}`, c]));
  const merged = staticList.map((s) => {
    const liveEntry = liveByKey.get(`${s.orientation}:${s.slug}`);
    if (!liveEntry) return s;
    return {
      ...s,
      videoCount: liveEntry.video_count ?? s.videoCount,
      categoryId: liveEntry.category_id ?? s.categoryId,
    };
  });
  for (const entry of live) {
    const key = `${entry.orientation}:${entry.slug}`;
    if (!staticList.some((s) => `${s.orientation}:${s.slug}` === key)) {
      merged.push({
        name: entry.name,
        slug: entry.slug,
        orientation: entry.orientation,
        categoryId: entry.category_id,
        videoCount: entry.video_count,
      });
    }
  }
  return merged.sort((a, b) => a.name.localeCompare(b.name));
}
