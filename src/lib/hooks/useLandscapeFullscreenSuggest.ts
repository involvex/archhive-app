import { useCallback, useState } from "react";
import { useIsLandscape } from "@/lib/hooks/useMediaQuery";
import {
  dismissLandscapeFullscreenSuggest,
  isLandscapeFullscreenSuggestDismissed,
  shouldOfferLandscapeFullscreen,
} from "@/lib/landscapeFullscreenSuggest";

/**
 * Shows a one-shot fullscreen offer when the viewport rotates into landscape
 * while the player is open (R08). Dismiss lasts for the browser session.
 */
export function useLandscapeFullscreenSuggest(
  enabled: boolean,
  isFullscreen: boolean,
): {
  showSuggest: boolean;
  accept: (enterFullscreen: () => void) => void;
  dismiss: () => void;
} {
  const isLandscape = useIsLandscape();
  const [dismissed, setDismissed] = useState(() => isLandscapeFullscreenSuggestDismissed());
  const [wasLandscape, setWasLandscape] = useState(isLandscape);
  const [showSuggest, setShowSuggest] = useState(false);

  // Adjust offer state while rendering when landscape / fullscreen inputs change
  // (React-supported "adjusting state when a prop changes" pattern).
  if (wasLandscape !== isLandscape) {
    const offer = shouldOfferLandscapeFullscreen({
      wasLandscape,
      isLandscape,
      isFullscreen: enabled ? isFullscreen : true,
      dismissed: dismissed || !enabled,
    });
    setWasLandscape(isLandscape);
    if (offer) setShowSuggest(true);
    else if (!isLandscape) setShowSuggest(false);
  } else if ((!enabled || isFullscreen) && showSuggest) {
    setShowSuggest(false);
  }

  const dismiss = useCallback(() => {
    dismissLandscapeFullscreenSuggest();
    setDismissed(true);
    setShowSuggest(false);
  }, []);

  const accept = useCallback((enterFullscreen: () => void) => {
    setShowSuggest(false);
    enterFullscreen();
  }, []);

  return { showSuggest: enabled && !isFullscreen && showSuggest, accept, dismiss };
}
