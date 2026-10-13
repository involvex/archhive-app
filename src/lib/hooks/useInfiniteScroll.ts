import { useCallback, useEffect, useRef, useState } from "react";

/** Pages loaded back to back while the sentinel stays on screen without the
 * user scrolling (a short page, or one mostly hidden by filters). Past this
 * the list waits for a scroll or a "Load more" tap instead of fetching page
 * after page from the site. */
const MAX_UNATTENDED_LOADS = 2;

export function useInfiniteScroll(onLoadMore: () => void, hasMore: boolean, loading: boolean) {
  const observerRef = useRef<IntersectionObserver | null>(null);
  const unattended = useRef(0);
  const [paused, setPaused] = useState(false);

  // Any scroll or touch by the user re-arms automatic loading.
  useEffect(() => {
    const onUser = () => {
      unattended.current = 0;
      setPaused(false);
    };
    const opts = { capture: true, passive: true } as const;
    for (const ev of ["wheel", "touchmove", "keydown"]) {
      window.addEventListener(ev, onUser, opts);
    }
    return () => {
      for (const ev of ["wheel", "touchmove", "keydown"]) {
        window.removeEventListener(ev, onUser, opts);
      }
    };
  }, []);

  const sentinelRef = useCallback(
    (node: HTMLDivElement | null) => {
      if (observerRef.current) {
        observerRef.current.disconnect();
        observerRef.current = null;
      }
      if (!node || !hasMore || loading || paused) return;
      const observer = new IntersectionObserver(
        (entries) => {
          if (!entries[0]?.isIntersecting || !hasMore || loading) return;
          if (unattended.current >= MAX_UNATTENDED_LOADS) {
            setPaused(true);
            return;
          }
          unattended.current += 1;
          onLoadMore();
        },
        { rootMargin: "300px" },
      );
      observer.observe(node);
      observerRef.current = observer;
    },
    [onLoadMore, hasMore, loading, paused],
  );

  useEffect(() => {
    return () => {
      if (observerRef.current) observerRef.current.disconnect();
    };
  }, []);

  /** Manual "Load more" (shown while `paused`). */
  const loadMore = useCallback(() => {
    unattended.current = 0;
    setPaused(false);
    onLoadMore();
  }, [onLoadMore]);

  return { sentinelRef, paused, loadMore };
}
