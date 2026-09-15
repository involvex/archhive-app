import { describe, it, expect } from "vitest";
import { applySceneQueryAndSort, isDurationRangeInvalid } from "@/lib/sceneList";
import type { Scene } from "@/lib/types";

const s = (id: string, title: string, extra: Partial<Scene> = {}): Scene => ({
  id,
  title,
  performers: [],
  tags: [],
  ...extra,
});

describe("isDurationRangeInvalid (Q8)", () => {
  it("flags min > max only", () => {
    expect(isDurationRangeInvalid(60, 30)).toBe(true);
    expect(isDurationRangeInvalid(30, 60)).toBe(false);
    expect(isDurationRangeInvalid(30, 30)).toBe(false);
    expect(isDurationRangeInvalid(undefined, 30)).toBe(false);
    expect(isDurationRangeInvalid(30, undefined)).toBe(false);
  });
});

describe("applySceneQueryAndSort (Q2+Q8)", () => {
  const scenes = [
    s("1", "Zulu dawn", { performers: ["Amy"], tags: ["outdoor"] }),
    s("2", "Alpha night", { performers: ["Zed"], tags: ["indoor"] }),
  ];

  it("filters by title, performer, and tag", () => {
    expect(applySceneQueryAndSort(scenes, "alpha", "newest").map((x) => x.id)).toEqual(["2"]);
    expect(applySceneQueryAndSort(scenes, "zed", "newest").map((x) => x.id)).toEqual(["2"]);
    expect(applySceneQueryAndSort(scenes, "outdoor", "newest").map((x) => x.id)).toEqual(["1"]);
  });

  it("applies name sort, preserves backend order otherwise", () => {
    expect(applySceneQueryAndSort(scenes, "", "name").map((x) => x.id)).toEqual(["2", "1"]);
    expect(applySceneQueryAndSort(scenes, "", "newest").map((x) => x.id)).toEqual(["1", "2"]);
    expect(applySceneQueryAndSort(scenes, "", "downloaded").map((x) => x.id)).toEqual(["1", "2"]);
  });
});
