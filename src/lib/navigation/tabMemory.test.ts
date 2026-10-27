import { beforeEach, describe, expect, it } from "vitest";
import {
  rememberLocation,
  resetTabMemory,
  savedScroll,
  saveScroll,
  tabAction,
  tabRootFor,
} from "./tabMemory";

const ROOTS = ["/", "/browse", "/library", "/live", "/downloads", "/settings"] as const;

describe("tab memory", () => {
  beforeEach(() => resetTabMemory());

  it("maps paths to their tab", () => {
    expect(tabRootFor("/", ROOTS)).toBe("/");
    expect(tabRootFor("/browse/xpomelo/search/milf", ROOTS)).toBe("/browse");
    expect(tabRootFor("/browser", ROOTS)).toBeUndefined();
    expect(tabRootFor("/library/scenes", ROOTS)).toBe("/library");
  });

  it("returns to the last location in another tab", () => {
    rememberLocation(ROOTS, "/browse/xpomelo/search/milf", "/browse/xpomelo/search/milf?sort=date");
    expect(tabAction("/browse", "/downloads", ROOTS)).toEqual({
      kind: "navigate",
      href: "/browse/xpomelo/search/milf?sort=date",
      restoreScroll: true,
    });
    // Unvisited tab: its start page.
    expect(tabAction("/library", "/downloads", ROOTS)).toEqual({
      kind: "navigate",
      href: "/library",
      restoreScroll: false,
    });
  });

  it("tapping the active tab goes to its start page, then to the top", () => {
    expect(tabAction("/browse", "/browse/pornhub/tag/milf", ROOTS)).toEqual({
      kind: "navigate",
      href: "/browse",
      restoreScroll: false,
    });
    expect(tabAction("/browse", "/browse", ROOTS)).toEqual({ kind: "scrollTop" });
  });

  it("keeps scroll per location", () => {
    saveScroll("/browse/a", 1234.6);
    expect(savedScroll("/browse/a")).toBe(1235);
    expect(savedScroll("/browse/b")).toBe(0);
  });
});
