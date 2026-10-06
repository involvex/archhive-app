import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "./tauri";

/**
 * Pull the pending Android share-target text (take + clear on the native side).
 * Returns trimmed text, or null when nothing was shared / not in the app runtime.
 */
export async function pollPendingShare(): Promise<string | null> {
  if (!isTauri()) return null;
  try {
    const text = await invoke<string | null>("get_pending_share");
    const trimmed = text?.trim();
    return trimmed ? trimmed : null;
  } catch {
    // Desktop builds have no share plugin yet (Android only) — stay quiet.
    return null;
  }
}
