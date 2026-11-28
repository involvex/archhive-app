/**
 * Saved-search JSON backup (R06). Portable listing definitions only —
 * runtime ids / new_count / last_checked_at are not restored.
 */

import type { BrowseFilters, BrowseKind, BrowseOrientation, SavedSearch } from "@/lib/types";

export const SAVED_SEARCH_BACKUP_SCHEMA = 1 as const;

const BROWSE_KINDS: readonly BrowseKind[] = [
  "tag",
  "model",
  "channel",
  "search",
  "video",
  "category",
  "livestream",
];

const ORIENTATIONS: readonly BrowseOrientation[] = ["straight", "gay", "lesbian", "transgender"];

const FILTER_KEYS = ["sort", "period", "duration", "quality", "include", "exclude", "tag"] as const;

/** One portable saved listing (no server-assigned id). */
export interface SavedSearchBackupEntry {
  name: string;
  site_id: string;
  kind: BrowseKind;
  slug: string;
  orientation?: BrowseOrientation;
  filters?: BrowseFilters;
  auto_queue?: boolean;
}

export interface SavedSearchBackup {
  schema_version: typeof SAVED_SEARCH_BACKUP_SCHEMA;
  exported_at: string;
  searches: SavedSearchBackupEntry[];
}

export interface SavedSearchImportResult {
  created: number;
  skipped: number;
  errors: string[];
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function optionalString(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function parseFilters(raw: unknown): BrowseFilters | undefined {
  if (raw === undefined || raw === null) return undefined;
  if (!isPlainObject(raw)) {
    throw new Error("filters must be an object");
  }
  const out: BrowseFilters = {};
  for (const key of FILTER_KEYS) {
    const v = raw[key];
    if (v === undefined || v === null) continue;
    if (typeof v !== "string") {
      throw new Error(`filters.${key} must be a string`);
    }
    if (v.trim()) out[key] = v;
  }
  return Object.keys(out).length > 0 ? out : undefined;
}

/** Validate and normalize one backup entry. Throws on invalid shape. */
export function parseSavedSearchEntry(raw: unknown, index: number): SavedSearchBackupEntry {
  const label = `searches[${index}]`;
  if (!isPlainObject(raw)) {
    throw new Error(`${label} must be an object`);
  }
  const name = typeof raw.name === "string" ? raw.name.trim() : "";
  const siteId = typeof raw.site_id === "string" ? raw.site_id.trim() : "";
  const slug = typeof raw.slug === "string" ? raw.slug.trim() : "";
  const kind = raw.kind;
  if (!name) throw new Error(`${label}.name is required`);
  if (!siteId) throw new Error(`${label}.site_id is required`);
  if (!slug) throw new Error(`${label}.slug is required`);
  if (typeof kind !== "string" || !(BROWSE_KINDS as readonly string[]).includes(kind)) {
    throw new Error(`${label}.kind must be a known BrowseKind`);
  }

  let orientation: BrowseOrientation | undefined;
  if (raw.orientation !== undefined && raw.orientation !== null) {
    if (
      typeof raw.orientation !== "string" ||
      !(ORIENTATIONS as readonly string[]).includes(raw.orientation)
    ) {
      throw new Error(`${label}.orientation is invalid`);
    }
    orientation = raw.orientation as BrowseOrientation;
  }

  let auto_queue: boolean | undefined;
  if (raw.auto_queue !== undefined && raw.auto_queue !== null) {
    if (typeof raw.auto_queue !== "boolean") {
      throw new Error(`${label}.auto_queue must be a boolean`);
    }
    auto_queue = raw.auto_queue;
  }

  const filters = parseFilters(raw.filters);

  return {
    name,
    site_id: siteId,
    kind: kind as BrowseKind,
    slug,
    orientation,
    filters,
    auto_queue,
  };
}

/** Parse a full backup document. Throws on invalid schema. */
export function parseSavedSearchBackup(raw: unknown): SavedSearchBackup {
  if (!isPlainObject(raw)) {
    throw new Error("Backup must be a JSON object");
  }
  if (raw.schema_version !== SAVED_SEARCH_BACKUP_SCHEMA) {
    throw new Error(
      `Unsupported schema_version (expected ${SAVED_SEARCH_BACKUP_SCHEMA}, got ${String(raw.schema_version)})`,
    );
  }
  if (!Array.isArray(raw.searches)) {
    throw new Error("searches must be an array");
  }
  const searches = raw.searches.map((entry, i) => parseSavedSearchEntry(entry, i));
  const exported_at = optionalString(raw.exported_at)?.trim() || new Date().toISOString();
  return {
    schema_version: SAVED_SEARCH_BACKUP_SCHEMA,
    exported_at,
    searches,
  };
}

/** Build an export document from the current saved-search list. */
export function buildSavedSearchBackup(searches: SavedSearch[]): SavedSearchBackup {
  return {
    schema_version: SAVED_SEARCH_BACKUP_SCHEMA,
    exported_at: new Date().toISOString(),
    searches: searches.map((s) => ({
      name: s.name,
      site_id: s.site_id,
      kind: s.kind,
      slug: s.slug,
      orientation: s.orientation,
      filters: s.filters,
      auto_queue: s.auto_queue,
    })),
  };
}

/** Match key used to skip duplicates on import (same listing identity). */
export function savedSearchIdentityKey(
  s: Pick<SavedSearchBackupEntry, "site_id" | "kind" | "slug" | "name">,
): string {
  return `${s.site_id}\0${s.kind}\0${s.slug}\0${s.name}`.toLowerCase();
}
