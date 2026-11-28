import { describe, expect, it } from "vitest";
import type { SavedSearch } from "@/lib/types";
import {
  buildSavedSearchBackup,
  parseSavedSearchBackup,
  parseSavedSearchEntry,
  savedSearchIdentityKey,
  SAVED_SEARCH_BACKUP_SCHEMA,
} from "@/lib/savedSearchBackup";

const sample: SavedSearch = {
  id: "abc",
  name: "Hot",
  site_id: "pornhub",
  kind: "search",
  slug: "milf",
  orientation: "straight",
  filters: { sort: "mr" },
  new_count: 3,
  auto_queue: true,
  created_at: "2026-01-01T00:00:00Z",
};

describe("savedSearchBackup (R06)", () => {
  it("builds a portable export without runtime fields", () => {
    const backup = buildSavedSearchBackup([sample]);
    expect(backup.schema_version).toBe(SAVED_SEARCH_BACKUP_SCHEMA);
    expect(backup.searches).toHaveLength(1);
    expect(backup.searches[0]).toEqual({
      name: "Hot",
      site_id: "pornhub",
      kind: "search",
      slug: "milf",
      orientation: "straight",
      filters: { sort: "mr" },
      auto_queue: true,
    });
    expect(backup.searches[0]).not.toHaveProperty("id");
    expect(backup.searches[0]).not.toHaveProperty("new_count");
  });

  it("round-trips a valid backup document", () => {
    const built = buildSavedSearchBackup([sample]);
    const parsed = parseSavedSearchBackup(JSON.parse(JSON.stringify(built)));
    expect(parsed.searches).toEqual(built.searches);
  });

  it("rejects wrong schema_version", () => {
    expect(() => parseSavedSearchBackup({ schema_version: 99, searches: [] })).toThrow(
      /schema_version/,
    );
  });

  it("rejects missing required fields", () => {
    expect(() => parseSavedSearchEntry({ name: "x" }, 0)).toThrow(/site_id/);
    expect(() =>
      parseSavedSearchEntry({ name: "x", site_id: "pornhub", kind: "nope", slug: "a" }, 0),
    ).toThrow(/BrowseKind/);
  });

  it("trims strings and drops empty filter values", () => {
    const entry = parseSavedSearchEntry(
      {
        name: "  Clip  ",
        site_id: " reddit ",
        kind: "tag",
        slug: " gonewild ",
        filters: { sort: "new", tag: "  ", include: "a" },
        auto_queue: false,
      },
      0,
    );
    expect(entry.name).toBe("Clip");
    expect(entry.site_id).toBe("reddit");
    expect(entry.slug).toBe("gonewild");
    expect(entry.filters).toEqual({ sort: "new", include: "a" });
  });

  it("builds a stable identity key for dedupe", () => {
    expect(savedSearchIdentityKey(sample)).toBe(
      savedSearchIdentityKey({
        name: "HOT",
        site_id: "Pornhub",
        kind: "search",
        slug: "milf",
      }),
    );
  });
});
