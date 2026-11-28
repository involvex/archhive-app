import type { AppSettings } from "@/lib/types";
import { inTimeWindow, localMinutesNow } from "@/lib/timeWindow";

export type NotifyEvent =
  | "download_complete"
  | "download_failed"
  | "saved_search_matches"
  | "watch_import"
  | "library_scan";

function eventEnabled(settings: AppSettings, event: NotifyEvent): boolean {
  switch (event) {
    case "download_complete":
      return settings.notify_download_complete !== false;
    case "download_failed":
      return settings.notify_download_failed !== false;
    case "saved_search_matches":
      return settings.notify_saved_search_matches !== false;
    case "watch_import":
      return settings.notify_watch_import !== false;
    case "library_scan":
      return settings.notify_library_scan !== false;
  }
}

function inQuietHours(settings: AppSettings, nowMinutes: number): boolean {
  if (!settings.quiet_hours_enabled) return false;
  return inTimeWindow(
    settings.quiet_hours_from ?? "22:00",
    settings.quiet_hours_to ?? "08:00",
    nowMinutes,
  );
}

/**
 * Whether an in-app toast / client-side notify should fire for `event`.
 * Quiet hours and per-event toggles both apply.
 */
export function shouldNotify(
  settings: AppSettings,
  event: NotifyEvent,
  nowMinutes: number = localMinutesNow(),
): boolean {
  if (!eventEnabled(settings, event)) return false;
  if (inQuietHours(settings, nowMinutes)) return false;
  return true;
}
