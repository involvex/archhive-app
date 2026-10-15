import { describe, expect, it } from "vitest";
import {
  buildAffinity,
  defaultRecPrefs,
  recommendScenes,
  resolveRecPrefs,
  type RecPrefs,
} from "./recommend";
import type { Scene, WatchProgress } from "./types";

function scene(partial: Partial<Scene> & { id: string }): Scene {
  return {
    title: partial.id,
    performers: [],
    tags: [],
    ...partial,
  };
}

function row(partial: Partial<WatchProgress> & { scene_id: string }): WatchProgress {
  return {
    position_secs: 0,
    duration_secs: 100,
    watched: false,
    updated_at: "2026-10-01T00:00:00Z",
    ...partial,
  };
}

const NOW = Date.parse("2026-10-15T00:00:00Z");

function prefs(overrides: Partial<RecPrefs> = {}): RecPrefs {
  return { ...defaultRecPrefs(), ...overrides };
}

describe("buildAffinity", () => {
  it("weights completed watches above partial playback", () => {
    const scenes = [
      scene({ id: "a", performers: ["Ann"] }),
      scene({ id: "b", performers: ["Bob"] }),
    ];
    const progress = [
      row({ scene_id: "a", watched: true, updated_at: "2026-10-14T00:00:00Z" }),
      row({ scene_id: "b", position_secs: 10, updated_at: "2026-10-14T00:00:00Z" }),
    ];
    const affinity = buildAffinity(scenes, progress, NOW);
    expect(affinity.performers.get("ann") ?? 0).toBeGreaterThan(
      affinity.performers.get("bob") ?? 0,
    );
  });

  it("decays older history", () => {
    const scenes = [scene({ id: "a", tags: ["x"] }), scene({ id: "b", tags: ["y"] })];
    const progress = [
      row({ scene_id: "a", watched: true, updated_at: "2026-10-14T00:00:00Z" }),
      row({ scene_id: "b", watched: true, updated_at: "2026-01-01T00:00:00Z" }),
    ];
    const affinity = buildAffinity(scenes, progress, NOW);
    expect(affinity.tags.get("x") ?? 0).toBeGreaterThan(affinity.tags.get("y") ?? 0);
  });

  it("ignores rows without playback and unknown scenes", () => {
    const affinity = buildAffinity(
      [scene({ id: "a" })],
      [row({ scene_id: "a" }), row({ scene_id: "ghost", watched: true })],
      NOW,
    );
    expect(affinity.performers.size + affinity.tags.size).toBe(0);
  });
});

describe("recommendScenes", () => {
  const library = [
    scene({ id: "watched", performers: ["Ann"], tags: ["solo"] }),
    scene({ id: "cand", performers: ["Ann"], tags: ["solo", "hd"] }),
    scene({ id: "other", performers: ["Zed"], tags: ["unrelated"] }),
  ];
  const history = [row({ scene_id: "watched", watched: true })];

  it("ranks affinity matches first with reasons", () => {
    const recs = recommendScenes(library, history, prefs(), NOW);
    expect(recs.map((r) => r.scene.id)).toEqual(["cand"]);
    expect(recs[0].reasons.join(" ")).toContain("Ann");
  });

  it("hides watched scenes when configured", () => {
    const recs = recommendScenes(library, history, prefs({ hideWatched: true }), NOW);
    expect(recs.some((r) => r.scene.id === "watched")).toBe(false);
  });

  it("keeps watched scenes when hideWatched is off", () => {
    const recs = recommendScenes(library, history, prefs({ hideWatched: false }), NOW);
    expect(recs.some((r) => r.scene.id === "watched")).toBe(true);
  });

  it("skips scenes with excluded tags", () => {
    const recs = recommendScenes(library, history, prefs({ excludedTags: ["hd"] }), NOW);
    expect(recs).toEqual([]);
  });

  it("boosts explicit preferences on a cold start", () => {
    const recs = recommendScenes(library, [], prefs({ preferredTags: ["unrelated"] }), NOW);
    expect(recs.map((r) => r.scene.id)).toEqual(["other"]);
    expect(recs[0].reasons.join(" ")).toContain("Preferred tag");
  });

  it("returns nothing on a cold start without preferences", () => {
    expect(recommendScenes(library, [], prefs(), NOW)).toEqual([]);
  });

  it("returns nothing when disabled and caps at maxItems", () => {
    expect(recommendScenes(library, history, prefs({ enabled: false }), NOW)).toEqual([]);
    const many = Array.from({ length: 10 }, (_, i) => scene({ id: `c${i}`, performers: ["Ann"] }));
    const recs = recommendScenes(
      [scene({ id: "w", performers: ["Ann"] }), ...many],
      [row({ scene_id: "w", watched: true })],
      prefs({ maxItems: 3 }),
      NOW,
    );
    expect(recs).toHaveLength(3);
  });
});

describe("resolveRecPrefs", () => {
  it("falls back to defaults for missing settings", () => {
    const resolved = resolveRecPrefs({} as never);
    expect(resolved).toEqual(defaultRecPrefs());
  });
});
