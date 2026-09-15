import { describe, it, expect, beforeEach } from "vitest";
import { sortPerformers, loadPerformerSort, savePerformerSort } from "@/lib/performerSort";
import type { Performer } from "@/lib/types";

const p = (name: string, scene_count: number): Performer => ({
  id: name,
  name,
  aliases: [],
  favorite: false,
  scene_count,
});

describe("sortPerformers (Q15)", () => {
  it("sorts by name", () => {
    const out = sortPerformers([p("Zed", 9), p("Amy", 1)], "name");
    expect(out.map((x) => x.name)).toEqual(["Amy", "Zed"]);
  });

  it("sorts by scene count desc with name tiebreak", () => {
    const out = sortPerformers([p("Zed", 2), p("Amy", 5), p("Bob", 2)], "scenes");
    expect(out.map((x) => x.name)).toEqual(["Amy", "Bob", "Zed"]);
  });

  it("does not mutate the input", () => {
    const input = [p("Zed", 1), p("Amy", 2)];
    sortPerformers(input, "scenes");
    expect(input.map((x) => x.name)).toEqual(["Zed", "Amy"]);
  });
});

describe("performer sort persistence (restart survival)", () => {
  beforeEach(() => localStorage.clear());

  it("defaults to name with nothing stored", () => {
    expect(loadPerformerSort()).toBe("name");
  });

  it("round-trips the saved sort (simulates restart: fresh load reads stored value)", () => {
    savePerformerSort("scenes");
    expect(loadPerformerSort()).toBe("scenes");
    savePerformerSort("name");
    expect(loadPerformerSort()).toBe("name");
  });

  it("falls back to name on corrupt stored value", () => {
    localStorage.setItem("archhive.performerSort", "bogus");
    expect(loadPerformerSort()).toBe("name");
  });
});
