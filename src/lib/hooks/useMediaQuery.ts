/**
 * useMediaQuery — reactive matchMedia subscription.
 *
 * Lightweight wrapper around `window.matchMedia` so components can react to
 * viewport orientation, hover capability, and motion preference without
 * sprinkling inline `@media` queries everywhere. Returns `false` during SSR
 * (no window) so the first paint is consistent.
 *
 * Mobile-first note: on touch devices `pointer: coarse` / `hover: none` are
 * the reliable way to detect "no mouse" — `navigator.maxTouchPoints` lies on
 * some Android WebViews.
 */

import { useEffect, useState } from "react";

export function useMediaQuery(query: string): boolean {
  // Lazily read the initial value so the effect only subscribes.
  const [matches, setMatches] = useState(
    () =>
      typeof window !== "undefined" &&
      typeof window.matchMedia === "function" &&
      window.matchMedia(query).matches,
  );

  useEffect(() => {
    if (typeof window === "undefined") return;
    const mql = window.matchMedia(query);
    // Older browsers (and some WebView shells) only expose `addEventListener`.
    const listener: (e: MediaQueryListEvent) => void = (e) => setMatches(e.matches);
    if (typeof mql.addEventListener === "function") {
      mql.addEventListener("change", listener);
      return () => mql.removeEventListener("change", listener);
    }
    // Fallback: legacy `addListener` (never reached on modern Chromium).
    mql.addListener(listener);
    return () => mql.removeListener(listener);
  }, [query]);

  return matches;
}

/** True when the viewport is in landscape orientation (wider than tall). */
export function useIsLandscape(): boolean {
  return useMediaQuery("(orientation: landscape) and (min-width: 641px)");
}

/** True on coarse-pointer (touch) devices — phones, tablets, Android WebView. */
export function useIsTouch(): boolean {
  return useMediaQuery("(pointer: coarse)");
}

/** True when the user has requested reduced motion (accessibility). */
export function usePrefersReducedMotion(): boolean {
  return useMediaQuery("(prefers-reduced-motion: reduce)");
}
