/**
 * Q44 AMOLED pure-black variant. Frontend-only overlay on top of the
 * existing `AppTheme` plumbing: when enabled, an `amoled` class on
 * <html> swaps the dark token values for pure black (see globals.css).
 * Persisted in localStorage so it survives restarts without a settings
 * migration; call `initAmoled()` once at startup (main.tsx).
 */

const STORAGE_KEY = "archhive_amoled";
const CLASS_NAME = "amoled";

export function isAmoled(): boolean {
  if (typeof window === "undefined" || typeof localStorage === "undefined") return false;
  try {
    return localStorage.getItem(STORAGE_KEY) === "1";
  } catch {
    return false;
  }
}

export function setAmoled(on: boolean): void {
  try {
    if (on) localStorage.setItem(STORAGE_KEY, "1");
    else localStorage.removeItem(STORAGE_KEY);
  } catch {
    /* storage unavailable — still toggle the class for this session */
  }
  if (typeof document !== "undefined") {
    document.documentElement.classList.toggle(CLASS_NAME, on);
  }
}

/** Apply the persisted choice before first paint. Idempotent. */
export function initAmoled(): void {
  if (typeof document === "undefined") return;
  document.documentElement.classList.toggle(CLASS_NAME, isAmoled());
}
