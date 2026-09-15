import { useEffect, useRef, useState } from "react";

interface WakeLockSentinelLike {
  release: () => Promise<void>;
}

interface WakeLockRequestLike {
  request: (type: "screen") => Promise<WakeLockSentinelLike>;
}

/** True when the Screen Wake Lock API is available (Q33). */
export function isWakeLockSupported(): boolean {
  if (typeof navigator === "undefined") return false;
  const wl = (navigator as Navigator & Partial<{ wakeLock: WakeLockRequestLike }>).wakeLock;
  return typeof wl?.request === "function";
}

/**
 * Request a screen wake lock, returning the sentinel or null on failure.
 * Split out so the request path is unit-testable without rendering the hook
 * (the vitest setup mocks `document.createElement`, which breaks renderHook).
 */
export async function requestScreenWakeLock(
  request: (type: "screen") => Promise<WakeLockSentinelLike>,
): Promise<WakeLockSentinelLike | null> {
  try {
    return await request("screen");
  } catch {
    return null;
  }
}

/**
 * Hold a screen wake lock while `active` (Q33). Requests on mount/activate,
 * re-acquires when the tab becomes visible again, and releases on cleanup.
 * Unsupported platforms and request failures degrade to `{ held: false }`.
 */
export function useWakeLock(active: boolean): { supported: boolean; held: boolean } {
  const [held, setHeld] = useState(false);
  const sentinelRef = useRef<WakeLockSentinelLike | null>(null);
  const supported = isWakeLockSupported();

  useEffect(() => {
    if (!active || !supported) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setHeld(false);
      return;
    }
    let cancelled = false;
    const release = () => {
      const s = sentinelRef.current;
      sentinelRef.current = null;
      if (s) s.release().catch(() => {});
    };
    const acquire = async () => {
      const s = await requestScreenWakeLock((t) =>
        (navigator as Navigator & { wakeLock: WakeLockRequestLike }).wakeLock.request(t),
      );
      if (cancelled) {
        s?.release().catch(() => {});
        return;
      }
      if (!s) {
        setHeld(false);
        return;
      }
      sentinelRef.current = s;
      setHeld(true);
    };
    const onVisibility = () => {
      if (document.visibilityState === "visible" && !cancelled && !sentinelRef.current) {
        void acquire();
      }
    };
    void acquire();
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      cancelled = true;
      document.removeEventListener("visibilitychange", onVisibility);
      release();
      setHeld(false);
    };
  }, [active, supported]);

  return { supported, held };
}
