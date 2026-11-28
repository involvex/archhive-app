/**
 * Post-wizard Home setup checklist (R07). Dismissed state is device-local;
 * completion is derived from AppSettings (+ optional cookie site count).
 */

import type { AppSettings } from "@/lib/types";
import type { SettingsTab } from "@/lib/settings/capabilities";

const STORAGE_KEY = "archhive.setupChecklist.dismissed";

export type SetupChecklistItemId = "library" | "engine" | "cookies" | "lan";

export interface SetupChecklistItem {
  id: SetupChecklistItemId;
  title: string;
  description: string;
  /** Settings tab to open. */
  tab: SettingsTab;
  done: boolean;
}

export interface SetupChecklistContext {
  settings: AppSettings;
  /** Sites that already have cookies in the vault (0 = none yet). */
  cookieSiteCount: number;
  /** True when this runtime can host the LAN server (desktop). */
  canHostLan: boolean;
}

export function isSetupChecklistDismissed(): boolean {
  if (typeof localStorage === "undefined") return false;
  try {
    return localStorage.getItem(STORAGE_KEY) === "1";
  } catch {
    return false;
  }
}

export function dismissSetupChecklist(): void {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(STORAGE_KEY, "1");
    }
  } catch {
    /* private mode / quota */
  }
}

export function clearSetupChecklistDismissed(): void {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.removeItem(STORAGE_KEY);
    }
  } catch {
    /* ignore */
  }
}

export function evaluateSetupChecklist(ctx: SetupChecklistContext): SetupChecklistItem[] {
  const { settings, cookieSiteCount, canHostLan } = ctx;
  const libraryDone = Boolean(settings.library_path?.trim());
  const engineDone = settings.engine_mode !== "remote_lan" || Boolean(settings.remote_host?.trim());
  const cookiesDone = cookieSiteCount > 0;
  const lanDone = canHostLan
    ? Boolean(settings.lan_enabled)
    : settings.engine_mode !== "remote_lan" || Boolean(settings.remote_host?.trim());

  return [
    {
      id: "library",
      title: "Confirm library path",
      description: "Where downloads and scene files are stored",
      tab: "library",
      done: libraryDone,
    },
    {
      id: "engine",
      title: "Choose download engine",
      description: "Local tools on this device, or Remote LAN to a desktop host",
      tab: "engine",
      done: engineDone,
    },
    {
      id: "cookies",
      title: "Import site cookies",
      description: "Needed for sites that require a login",
      tab: "cookies",
      done: cookiesDone,
    },
    {
      id: "lan",
      title: canHostLan ? "Enable LAN for mobile" : "Connect to desktop LAN",
      description: canHostLan
        ? "Let phones browse and download through this PC"
        : "Point Remote LAN at your desktop ArcHive (:8787)",
      tab: "lan",
      done: lanDone,
    },
  ];
}

/** Show after the first-run wizard until dismissed or every item is done. */
export function shouldShowSetupChecklist(
  wizardCompleted: boolean | undefined,
  items: SetupChecklistItem[],
  dismissed: boolean = isSetupChecklistDismissed(),
): boolean {
  if (!wizardCompleted) return false;
  if (dismissed) return false;
  return items.some((i) => !i.done);
}
