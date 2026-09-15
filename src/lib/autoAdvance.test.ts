import { describe, it, expect } from "vitest";
import { getAutoAdvanceTarget } from "@/lib/autoAdvance";

describe("getAutoAdvanceTarget (Q24)", () => {
  const scenes = ["a", "b", "c"];

  it("returns the next scene when enabled", () => {
    expect(getAutoAdvanceTarget(scenes, 0, true)).toEqual({ scene: "b", index: 1 });
  });

  it("returns null at the end of the queue", () => {
    expect(getAutoAdvanceTarget(scenes, 2, true)).toBeNull();
  });

  it("returns null when disabled", () => {
    expect(getAutoAdvanceTarget(scenes, 0, false)).toBeNull();
  });

  it("returns null without a valid index", () => {
    expect(getAutoAdvanceTarget(scenes, undefined, true)).toBeNull();
    expect(getAutoAdvanceTarget(scenes, null, true)).toBeNull();
    expect(getAutoAdvanceTarget(undefined, 0, true)).toBeNull();
  });
});
