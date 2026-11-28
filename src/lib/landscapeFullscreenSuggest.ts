/**
 * R08 — when the player is open and the viewport rotates into landscape,
 * offer fullscreen once (dismissible for the session).
 */

const DISMISS_KEY = "archhive.player.landscapeFs.dismissed";

export function isLandscapeFullscreenSuggestDismissed(): boolean {
  if (typeof sessionStorage === "undefined") return false;
  try {
    return sessionStorage.getItem(DISMISS_KEY) === "1";
  } catch {
    return false;
  }
}

export function dismissLandscapeFullscreenSuggest(): void {
  try {
    if (typeof sessionStorage !== "undefined") {
      sessionStorage.setItem(DISMISS_KEY, "1");
    }
  } catch {
    /* private mode / quota */
  }
}

export function clearLandscapeFullscreenSuggestDismissed(): void {
  try {
    if (typeof sessionStorage !== "undefined") {
      sessionStorage.removeItem(DISMISS_KEY);
    }
  } catch {
    /* ignore */
  }
}

/** Rising edge into landscape while not fullscreen and not dismissed. */
export function shouldOfferLandscapeFullscreen(opts: {
  wasLandscape: boolean;
  isLandscape: boolean;
  isFullscreen: boolean;
  dismissed: boolean;
}): boolean {
  if (opts.dismissed || opts.isFullscreen || !opts.isLandscape) return false;
  return !opts.wasLandscape;
}
