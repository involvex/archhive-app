import { describe, expect, it } from "vitest";
import { nextChipState } from "./CategoryIncludeExclude";

describe("nextChipState", () => {
  it("cycles neutral → include → exclude → neutral", () => {
    expect(nextChipState("none", true, true)).toBe("include");
    expect(nextChipState("include", true, true)).toBe("exclude");
    expect(nextChipState("exclude", true, true)).toBe("none");
  });

  it("goes straight to exclude when include is full or unavailable", () => {
    expect(nextChipState("none", false, true)).toBe("exclude");
  });

  it("skips exclude when the exclude list is full", () => {
    expect(nextChipState("include", true, false)).toBe("none");
    expect(nextChipState("none", false, false)).toBe("none");
  });
});
