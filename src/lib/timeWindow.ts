/** Parse "HH:MM" into minutes-since-midnight. Returns null on bad input. */
export function parseTimeHM(time: string): number | null {
  const parts = time.trim().split(":");
  if (parts.length !== 2) return null;
  const h = Number(parts[0]);
  const m = Number(parts[1]);
  if (!Number.isInteger(h) || !Number.isInteger(m) || h < 0 || h > 23 || m < 0 || m > 59) {
    return null;
  }
  return h * 60 + m;
}

/**
 * True when `nowMinutes` is inside `[from, to)` with overnight wrap
 * (same semantics as scheduled theme / quiet hours).
 */
export function inTimeWindow(from: string, to: string, nowMinutes: number): boolean {
  const fromMin = parseTimeHM(from);
  const toMin = parseTimeHM(to);
  if (fromMin == null || toMin == null) return false;
  if (fromMin <= toMin) {
    return nowMinutes >= fromMin && nowMinutes < toMin;
  }
  return nowMinutes >= fromMin || nowMinutes < toMin;
}

/** Local wall-clock minutes since midnight. */
export function localMinutesNow(date: Date = new Date()): number {
  return date.getHours() * 60 + date.getMinutes();
}
