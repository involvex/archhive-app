/**
 * Touch-gesture math for the mobile player (#44). Pure functions so the
 * thresholds are unit-tested; the component only wires DOM touch events.
 * Gestures never call preventDefault — native controls and vertical scroll
 * keep working (the wrapper uses `touch-action: pan-y`).
 */

export interface TouchPoint {
  x: number;
  y: number;
  t: number;
}

/** Horizontal travel that counts as a scrub (below this is a tap). */
export const SWIPE_MIN_PX = 24;
/** Max gap between two taps to count as a double-tap. */
export const DOUBLE_TAP_MS = 300;
/** Max finger travel that still counts as a tap. */
export const TAP_MAX_PX = 12;
/** Seconds skipped by a double-tap on the left/right third. */
export const SKIP_SECONDS = 10;

export type TapZone = "left" | "middle" | "right";

export function tapZone(x: number, width: number): TapZone {
  if (width <= 0) return "middle";
  if (x < width / 3) return "left";
  if (x > (width * 2) / 3) return "right";
  return "middle";
}

/** True when `cur` is a second tap quickly following `prev` in place. */
export function isDoubleTap(prev: TouchPoint | null, cur: TouchPoint): boolean {
  if (!prev) return false;
  return (
    cur.t - prev.t <= DOUBLE_TAP_MS && Math.hypot(cur.x - prev.x, cur.y - prev.y) <= TAP_MAX_PX
  );
}

/** True once a drag is far enough — and horizontal enough — to be a scrub. */
export function isScrub(dxPx: number, dyPx: number): boolean {
  return Math.abs(dxPx) >= SWIPE_MIN_PX && Math.abs(dxPx) > Math.abs(dyPx) * 1.2;
}

/**
 * Scrub target: a full-width swipe traverses the whole video.
 * Clamped to [0, duration]; NaN-safe for unknown durations.
 */
export function swipeSeekTarget(
  startTime: number,
  dxPx: number,
  widthPx: number,
  duration: number,
): number {
  if (!Number.isFinite(duration) || duration <= 0 || !Number.isFinite(widthPx) || widthPx <= 0) {
    return startTime;
  }
  const target = startTime + (dxPx / widthPx) * duration;
  return Math.min(duration, Math.max(0, target));
}

/** "+10s" / "−10s" overlay label for a skip. */
export function formatSkipLabel(seconds: number): string {
  return seconds < 0 ? `−${Math.abs(Math.round(seconds))}s` : `+${Math.round(seconds)}s`;
}
