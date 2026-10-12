import { describe, expect, it } from "vitest";
import { applyLocalFilters, EMPTY_LOCAL_FILTERS } from "./BrowseFilterBar";
import { browseFiltersKey } from "@/lib/api/client";

const items = [
  { title: "Amateur couple at home", duration: 200, tags: ["homemade"] },
  { title: "Long amateur compilation", duration: 3600, channel: "Studio X" },
  { title: "Unknown length clip", duration: undefined },
];

describe("applyLocalFilters", () => {
  it("passes everything through when empty", () => {
    expect(applyLocalFilters(items, EMPTY_LOCAL_FILTERS)).toHaveLength(3);
  });

  it("matches all words and honours -exclusions across title/channel/tags", () => {
    const hit = applyLocalFilters(items, { text: "amateur -compilation", duration: "" });
    expect(hit.map((i) => i.title)).toEqual(["Amateur couple at home"]);
    expect(applyLocalFilters(items, { text: "studio", duration: "" })).toHaveLength(1);
    expect(applyLocalFilters(items, { text: "homemade", duration: "" })).toHaveLength(1);
  });

  it("filters by duration bucket but keeps items without a duration", () => {
    const long = applyLocalFilters(items, { text: "", duration: "long" });
    expect(long.map((i) => i.title)).toEqual(["Long amateur compilation", "Unknown length clip"]);
  });
});

describe("browseFiltersKey", () => {
  it("mirrors the Rust cache key", () => {
    expect(browseFiltersKey({})).toBe("");
    expect(browseFiltersKey({ sort: "mv", quality: "hd" })).toBe("|s=mv|p=|d=|q=hd");
    expect(browseFiltersKey({ include: "milf", exclude: "anal" })).toBe(
      "|s=|p=|d=|q=|i=milf|x=anal",
    );
    expect(browseFiltersKey({ tag: "anal" })).toBe("|s=|p=|d=|q=|t=anal");
  });
});
