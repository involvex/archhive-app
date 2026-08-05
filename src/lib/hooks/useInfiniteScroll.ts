import { useCallback, useEffect, useRef } from "react";

export function useInfiniteScroll(onLoadMore: () => void, hasMore: boolean, loading: boolean) {
  const observerRef = useRef<IntersectionObserver | null>(null);
  const sentinelRef = useCallback(
    (node: HTMLDivElement | null) => {
      if (observerRef.current) {
        observerRef.current.disconnect();
        observerRef.current = null;
      }
      if (!node || !hasMore || loading) return;
      const observer = new IntersectionObserver(
        (entries) => {
          if (entries[0]?.isIntersecting && hasMore && !loading) {
            onLoadMore();
          }
        },
        { rootMargin: "300px" },
      );
      observer.observe(node);
      observerRef.current = observer;
    },
    [onLoadMore, hasMore, loading],
  );

  useEffect(() => {
    return () => {
      if (observerRef.current) observerRef.current.disconnect();
    };
  }, []);

  return { sentinelRef };
}
