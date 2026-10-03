import { useCallback, useRef, useState } from "react";

export interface UseRefreshGuardResult {
  /** Stable callback to hand to `usePullToRefresh({ onRefresh })`. */
  refresh: () => Promise<void>;
  /** True while a refresh is in flight — lets callers disable the affordance. */
  refreshing: boolean;
}

/**
 * Wraps a refresh routine with an in-flight guard so repeated pull-to-refresh
 * gestures (or a mount effect plus a manual pull) cannot stack duplicate scrape
 * waves. On Android this is the difference between one request and a burst that
 * gets the host rate-limiting us.
 *
 * Re-entrant calls resolve against the already-running refresh instead of
 * starting a second one.
 */
export function useRefreshGuard(fn: () => Promise<unknown>): UseRefreshGuardResult {
  const inFlight = useRef<Promise<void> | null>(null);
  const [refreshing, setRefreshing] = useState(false);

  const refresh = useCallback(async () => {
    if (inFlight.current) return inFlight.current;
    setRefreshing(true);
    const run: Promise<void> = fn()
      .then(() => undefined)
      .catch((e: unknown) => {
        console.error("refresh failed", e);
      })
      .finally(() => {
        inFlight.current = null;
        setRefreshing(false);
      });
    inFlight.current = run;
    return run;
  }, [fn]);

  return { refresh, refreshing };
}
