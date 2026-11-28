import { beforeEach, describe, expect, it } from "vitest";
import {
  clearLandscapeFullscreenSuggestDismissed,
  dismissLandscapeFullscreenSuggest,
  isLandscapeFullscreenSuggestDismissed,
  shouldOfferLandscapeFullscreen,
} from "@/lib/landscapeFullscreenSuggest";

describe("landscapeFullscreenSuggest (R08)", () => {
  beforeEach(() => {
    clearLandscapeFullscreenSuggestDismissed();
    sessionStorage.clear();
  });

  it("offers only on rising edge into landscape", () => {
    expect(
      shouldOfferLandscapeFullscreen({
        wasLandscape: false,
        isLandscape: true,
        isFullscreen: false,
        dismissed: false,
      }),
    ).toBe(true);

    expect(
      shouldOfferLandscapeFullscreen({
        wasLandscape: true,
        isLandscape: true,
        isFullscreen: false,
        dismissed: false,
      }),
    ).toBe(false);

    expect(
      shouldOfferLandscapeFullscreen({
        wasLandscape: false,
        isLandscape: false,
        isFullscreen: false,
        dismissed: false,
      }),
    ).toBe(false);
  });

  it("skips when fullscreen or dismissed", () => {
    expect(
      shouldOfferLandscapeFullscreen({
        wasLandscape: false,
        isLandscape: true,
        isFullscreen: true,
        dismissed: false,
      }),
    ).toBe(false);
    expect(
      shouldOfferLandscapeFullscreen({
        wasLandscape: false,
        isLandscape: true,
        isFullscreen: false,
        dismissed: true,
      }),
    ).toBe(false);
  });

  it("persists dismiss in sessionStorage", () => {
    expect(isLandscapeFullscreenSuggestDismissed()).toBe(false);
    dismissLandscapeFullscreenSuggest();
    expect(isLandscapeFullscreenSuggestDismissed()).toBe(true);
  });
});
