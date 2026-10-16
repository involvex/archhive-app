import type { AppSettings, MediaItem, Scene, WatchProgress } from "./types";

/** Resolved recommendation preferences (all fields concrete). */
export interface RecPrefs {
  enabled: boolean;
  hideWatched: boolean;
  includeLive: boolean;
  includeFeed: boolean;
  maxItems: number;
  preferredTags: string[];
  preferredPerformers: string[];
  excludedTags: string[];
}

export function defaultRecPrefs(): RecPrefs {
  return {
    enabled: true,
    hideWatched: true,
    includeLive: true,
    includeFeed: true,
    maxItems: 24,
    preferredTags: [],
    preferredPerformers: [],
    excludedTags: [],
  };
}

function asList(value: string[] | undefined): string[] {
  return Array.isArray(value) ? value : [];
}

/** Merge stored settings over the defaults (Phase 1: library picks). */
export function resolveRecPrefs(settings: AppSettings): RecPrefs {
  const d = defaultRecPrefs();
  return {
    enabled: settings.rec_enabled ?? d.enabled,
    hideWatched: settings.rec_hide_watched ?? d.hideWatched,
    includeLive: settings.rec_include_live ?? d.includeLive,
    includeFeed: settings.rec_include_feed ?? d.includeFeed,
    maxItems: settings.rec_max_items ?? d.maxItems,
    preferredTags: asList(settings.rec_preferred_tags),
    preferredPerformers: asList(settings.rec_preferred_performers),
    excludedTags: asList(settings.rec_excluded_tags),
  };
}

/** Weighted interest profile built from watch history. */
export interface Affinity {
  performers: Map<string, number>;
  tags: Map<string, number>;
  studios: Map<string, number>;
  sites: Map<string, number>;
}

export function emptyAffinity(): Affinity {
  return { performers: new Map(), tags: new Map(), studios: new Map(), sites: new Map() };
}

function addWeighted(map: Map<string, number>, key: string | undefined, weight: number) {
  const k = (key ?? "").trim().toLowerCase();
  if (!k || weight <= 0) return;
  map.set(k, (map.get(k) ?? 0) + weight);
}

function parseTime(value: string | undefined, fallbackMs: number): number {
  if (!value) return fallbackMs;
  const t = Date.parse(value);
  return Number.isFinite(t) ? t : fallbackMs;
}

/**
 * Build an affinity profile from watch history.
 * Each history row contributes by completion (watched = 1, else
 * position/duration clamped to 0..1) with exponential recency decay
 * (30-day half-life scale). Rows with no playback are ignored.
 */
export function buildAffinity(
  scenes: Scene[],
  progress: WatchProgress[],
  nowMs: number = Date.now(),
): Affinity {
  const affinity = emptyAffinity();
  const byId = new Map(scenes.map((s) => [s.id, s]));
  for (const row of progress) {
    const scene = byId.get(row.scene_id);
    if (!scene) continue;
    const duration = row.duration_secs > 0 ? row.duration_secs : 0;
    const completion = row.watched ? 1 : duration > 0 ? row.position_secs / duration : 0;
    if (!(completion > 0)) continue;
    const ageDays = Math.max(0, (nowMs - parseTime(row.updated_at, nowMs)) / 86_400_000);
    const signal = (0.3 + 0.7 * Math.min(1, completion)) * Math.exp(-ageDays / 30);
    for (const p of scene.performers) addWeighted(affinity.performers, p, signal);
    for (const t of scene.tags) addWeighted(affinity.tags, t, signal);
    addWeighted(affinity.studios, scene.studio_name ?? scene.studio_id, signal);
    addWeighted(affinity.sites, scene.channel, signal);
  }
  return affinity;
}

export interface Recommendation {
  scene: Scene;
  score: number;
  reasons: string[];
}

const WEIGHTS = {
  performer: 3,
  tag: 2,
  studio: 2,
  site: 1,
  preferredPerformer: 5,
  preferredTag: 3,
} as const;

function lowerSet(values: string[]): Set<string> {
  return new Set(values.map((v) => v.trim().toLowerCase()).filter(Boolean));
}

/** Structural subset shared by library scenes and browse/live items. */
interface Scorable {
  performers: string[];
  tags: string[];
  channel?: string;
  studio_name?: string;
  studio_id?: string;
}

interface Scored {
  score: number;
  reasons: string[];
}

/** Affinity + preference overlap for one candidate; null when no signal. */
function scoreCommon(
  item: Scorable,
  affinity: Affinity,
  preferredTags: Set<string>,
  preferredPerformers: Set<string>,
): Scored | null {
  const reasons: string[] = [];
  let score = 0;
  for (const p of item.performers) {
    const key = p.trim().toLowerCase();
    const w = affinity.performers.get(key) ?? 0;
    if (w > 0) {
      score += WEIGHTS.performer * w;
      reasons.push(`Performer "${p}"`);
    } else if (preferredPerformers.has(key)) {
      score += WEIGHTS.preferredPerformer;
      reasons.push(`Preferred performer "${p}"`);
    }
  }
  for (const t of item.tags) {
    const key = t.trim().toLowerCase();
    const w = affinity.tags.get(key) ?? 0;
    if (w > 0) {
      score += WEIGHTS.tag * w;
      reasons.push(`Tag "${t}"`);
    } else if (preferredTags.has(key)) {
      score += WEIGHTS.preferredTag;
      reasons.push(`Preferred tag "${t}"`);
    }
  }
  const studioKey = (item.studio_name ?? item.studio_id ?? "").trim().toLowerCase();
  const studioW = studioKey ? (affinity.studios.get(studioKey) ?? 0) : 0;
  if (studioW > 0) {
    score += WEIGHTS.studio * studioW;
    reasons.push(`Studio "${item.studio_name ?? item.studio_id}"`);
  }
  const siteKey = (item.channel ?? "").trim().toLowerCase();
  const siteW = siteKey ? (affinity.sites.get(siteKey) ?? 0) : 0;
  if (siteW > 0) score += WEIGHTS.site * siteW;
  if (score <= 0) return null;
  return { score, reasons: reasons.slice(0, 2) };
}

/**
 * Rank unwatched library scenes by affinity overlap plus explicit
 * preferences. Returns at most `prefs.maxItems` entries, best first.
 * An empty affinity (no usable history) falls back to explicit
 * preferences; with neither, no recommendations are produced.
 */
export function recommendScenes(
  scenes: Scene[],
  progress: WatchProgress[],
  prefs: RecPrefs,
  nowMs: number = Date.now(),
): Recommendation[] {
  if (!prefs.enabled || prefs.maxItems <= 0) return [];
  const watchedIds = new Set(progress.filter((r) => r.watched).map((r) => r.scene_id));
  const affinity = buildAffinity(scenes, progress, nowMs);
  const hasAffinity = affinity.performers.size + affinity.tags.size + affinity.studios.size > 0;
  const preferredTags = lowerSet(prefs.preferredTags);
  const preferredPerformers = lowerSet(prefs.preferredPerformers);
  const excludedTags = lowerSet(prefs.excludedTags);
  if (!hasAffinity && preferredTags.size + preferredPerformers.size === 0) return [];

  const out: Recommendation[] = [];
  for (const scene of scenes) {
    if (prefs.hideWatched && watchedIds.has(scene.id)) continue;
    if (scene.tags.some((t) => excludedTags.has(t.trim().toLowerCase()))) continue;
    const scored = scoreCommon(scene, affinity, preferredTags, preferredPerformers);
    if (!scored) continue;
    let { score } = scored;
    if (!prefs.hideWatched && watchedIds.has(scene.id)) score *= 0.5;
    out.push({ scene, score, reasons: scored.reasons });
  }
  out.sort((a, b) => b.score - a.score || a.scene.title.localeCompare(b.scene.title));
  return out.slice(0, prefs.maxItems);
}

export interface RankedItem {
  item: MediaItem;
  score: number;
  reasons: string[];
}

/**
 * Rank browse/live items (live rooms, saved-search matches, trending) by
 * the same affinity profile. Viewers only break ties — affinity dominates.
 * Items whose tags are all excluded are skipped.
 */
export function rankMediaItems(
  items: MediaItem[],
  affinity: Affinity,
  prefs: RecPrefs,
  maxItems: number = 12,
): RankedItem[] {
  if (!prefs.enabled || maxItems <= 0) return [];
  const preferredTags = lowerSet(prefs.preferredTags);
  const preferredPerformers = lowerSet(prefs.preferredPerformers);
  const excludedTags = lowerSet(prefs.excludedTags);
  const seen = new Set<string>();
  const out: RankedItem[] = [];
  for (const item of items) {
    const key = item.url || item.id;
    if (seen.has(key)) continue;
    seen.add(key);
    if (item.tags.some((t) => excludedTags.has(t.trim().toLowerCase()))) continue;
    const scored = scoreCommon(item, affinity, preferredTags, preferredPerformers);
    if (!scored) continue;
    const tiebreak = Math.log10((item.viewers ?? 0) + 1) * 0.1;
    out.push({ item, score: scored.score + tiebreak, reasons: scored.reasons });
  }
  out.sort((a, b) => b.score - a.score || a.item.title.localeCompare(b.item.title));
  return out.slice(0, maxItems);
}
