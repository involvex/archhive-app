/**
 * App Lock helpers (R11). Pure timing logic so cold-start / background return
 * can be unit-tested without the DOM.
 */

export function shouldEngageAppLock(opts: {
  enabled: boolean;
  pinConfigured: boolean;
  /** Epoch ms when the app last went to background; null = cold start. */
  hiddenAt: number | null;
  /** Epoch ms when the app became visible again (or now on cold start). */
  now: number;
  timeoutMins: number;
}): boolean {
  if (!opts.enabled || !opts.pinConfigured) return false;
  // Cold start — always lock when App Lock is on.
  if (opts.hiddenAt == null) return true;
  const timeoutMs = Math.max(0, opts.timeoutMins) * 60_000;
  return opts.now - opts.hiddenAt >= timeoutMs;
}

export function hasVaultPinConfigured(vaultPinHash: string | undefined | null): boolean {
  return Boolean(vaultPinHash?.trim());
}
