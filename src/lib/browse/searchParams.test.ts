import { describe, expect, it } from "vitest";
import { filtersFromSearch, parseBrowseSearch } from "./searchParams";

describe("parseBrowseSearch", () => {
  it("keeps valid orientation and non-empty filters", () => {
    const s = parseBrowseSearch({
      orientation: "gay",
      sort: "mv",
      exclude: "35,8",
      period: "  ",
      bogus: "x",
    });
    expect(s).toEqual({ orientation: "gay", sort: "mv", exclude: "35,8" });
    expect(filtersFromSearch(s)).toEqual({ sort: "mv", exclude: "35,8" });
  });

  it("drops unknown orientations and non-scalar values", () => {
    expect(parseBrowseSearch({ orientation: "other", sort: { a: 1 } })).toEqual({});
  });

  it("turns numeric ids parsed from the URL back into strings", () => {
    expect(parseBrowseSearch({ exclude: 35 })).toEqual({ exclude: "35" });
  });
});
