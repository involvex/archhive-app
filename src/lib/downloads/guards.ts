/**
 * Download guards for mobile: metered-network confirmation + free-space warning.
 *
 * Pure logic lives here so both the Downloads page and the bulk-import panel
 * share one policy. UI (the confirm dialog) is wired by callers.
 */

export const STORAGE_WARN_BYTES = 1024 * 1024 * 1024; // 1 GiB
export const STORAGE_CRITICAL_BYTES = 256 * 1024 * 1024; // 256 MiB

/** True when the active connection is metered (cellular / save-data / offline). */
export function isMeteredConnection(): boolean {
  if (typeof navigator === "undefined") return false;
  if (!navigator.onLine) return true;
  const nav = navigator as Navigator & {
    connection?: { type?: string; effectiveType?: string; saveData?: boolean };
  };
  const conn = nav.connection;
  if (!conn) return false;
  if (conn.saveData) return true;
  const raw = (conn.type || conn.effectiveType || "").toLowerCase();
  return (
    raw === "cellular" ||
    raw === "wimax" ||
    raw.startsWith("2g") ||
    raw.startsWith("3g") ||
    raw.startsWith("4g") ||
    raw.startsWith("5g") ||
    raw === "slow-2g"
  );
}

export type GuardBlock =
  | { kind: "cellular"; metered: true }
  | { kind: "storage"; freeBytes: number; critical: boolean }
  | null;

/**
 * Decide whether a download action needs user confirmation.
 * Returns the first blocking reason, or null when clear to proceed.
 */
export function checkDownloadGuards(opts: {
  wifiOnly: boolean;
  freeBytes: number | null;
}): GuardBlock {
  if (opts.wifiOnly && isMeteredConnection()) {
    return { kind: "cellular", metered: true };
  }
  if (opts.freeBytes != null && opts.freeBytes < STORAGE_WARN_BYTES) {
    return {
      kind: "storage",
      freeBytes: opts.freeBytes,
      critical: opts.freeBytes < STORAGE_CRITICAL_BYTES,
    };
  }
  return null;
}

export function formatBytes(bytes: number): string {
  if (bytes >= 1073741824) return `${(bytes / 1073741824).toFixed(1)} GB`;
  if (bytes >= 1048576) return `${(bytes / 1048576).toFixed(0)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${bytes} B`;
}
