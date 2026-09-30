import { useCallback, useEffect, useRef, useState } from "react";
import { vibrateTick } from "@/lib/haptics";

interface PullToRefreshOptions {
  onRefresh: () => Promise<void>;
  disabled?: boolean;
  threshold?: number;
}

export function usePullToRefresh({
  onRefresh,
  disabled = false,
  threshold = 80,
}: PullToRefreshOptions) {
  const [pullDistance, setPullDistance] = useState(0);
  const [refreshing, setRefreshing] = useState(false);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const startY = useRef(0);
  const pulling = useRef(false);
  const rafId = useRef(0);

  const isAtTop = () => {
    // Android WebView reports scroll on body/window, not documentElement.
    if (typeof window !== "undefined" && window.scrollY > 0) return false;
    if (typeof document !== "undefined" && document.documentElement.scrollTop > 0) return false;
    if (typeof document !== "undefined" && document.body.scrollTop > 0) return false;
    return true;
  };

  const handleTouchStart = useCallback(
    (e: TouchEvent) => {
      if (disabled || !isAtTop()) return;
      startY.current = e.touches[0].clientY;
      pulling.current = true;
    },
    [disabled],
  );

  const handleTouchMove = useCallback(
    (e: TouchEvent) => {
      if (!pulling.current) return;
      const delta = e.touches[0].clientY - startY.current;
      if (delta > 0) {
        // Coalesce via rAF: touchmove fires faster than React can render,
        // and setState per event janks scrolling on low-end Android.
        if (rafId.current) return;
        const target = Math.min(delta * 0.4, threshold * 1.5);
        rafId.current = requestAnimationFrame(() => {
          rafId.current = 0;
          setPullDistance(target);
        });
      }
    },
    [threshold],
  );

  const handleTouchEnd = useCallback(() => {
    if (!pulling.current) return;
    pulling.current = false;
    if (pullDistance >= threshold && !refreshing) {
      setRefreshing(true);
      // Q39: haptic tick as the refresh engages.
      vibrateTick(10);
      void onRefresh().finally(() => {
        setRefreshing(false);
        setPullDistance(0);
      });
    } else {
      setPullDistance(0);
    }
  }, [pullDistance, threshold, refreshing, onRefresh]);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    el.addEventListener("touchstart", handleTouchStart, { passive: true });
    el.addEventListener("touchmove", handleTouchMove, { passive: true });
    el.addEventListener("touchend", handleTouchEnd);
    return () => {
      el.removeEventListener("touchstart", handleTouchStart);
      el.removeEventListener("touchmove", handleTouchMove);
      el.removeEventListener("touchend", handleTouchEnd);
    };
  }, [handleTouchStart, handleTouchMove, handleTouchEnd]);

  return { containerRef, pullDistance, refreshing };
}
