/**
 * Central haptic helper (Q39). Wraps `navigator.vibrate` so call sites
 * (queue-add, long-press, pull-refresh) share one guarded entry point.
 * No-op on desktop browsers and platforms without vibration support.
 *
 * @param pattern Vibration pattern in ms (default 10). Returns true when
 * a vibration was actually requested.
 */
export function vibrateTick(pattern: number | number[] = 10): boolean {
  if (typeof navigator === "undefined") return false;
  try {
    const vibrate = navigator.vibrate?.bind(navigator);
    if (!vibrate) return false;
    return vibrate(pattern);
  } catch {
    return false;
  }
}
