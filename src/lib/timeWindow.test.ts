import { describe, expect, it } from "vitest";
import { inTimeWindow, parseTimeHM } from "@/lib/timeWindow";
import { shouldNotify } from "@/lib/shouldNotify";
import type { AppSettings } from "@/lib/types";

describe("parseTimeHM", () => {
  it("parses valid times", () => {
    expect(parseTimeHM("22:00")).toBe(22 * 60);
    expect(parseTimeHM("08:30")).toBe(8 * 60 + 30);
    expect(parseTimeHM("0:00")).toBe(0);
  });

  it("rejects bad input", () => {
    expect(parseTimeHM("")).toBeNull();
    expect(parseTimeHM("25:00")).toBeNull();
    expect(parseTimeHM("12:60")).toBeNull();
    expect(parseTimeHM("noon")).toBeNull();
  });
});

describe("inTimeWindow", () => {
  it("handles same-day windows", () => {
    expect(inTimeWindow("09:00", "17:00", 10 * 60)).toBe(true);
    expect(inTimeWindow("09:00", "17:00", 8 * 60)).toBe(false);
    expect(inTimeWindow("09:00", "17:00", 17 * 60)).toBe(false);
  });

  it("handles overnight wrap", () => {
    expect(inTimeWindow("22:00", "08:00", 23 * 60)).toBe(true);
    expect(inTimeWindow("22:00", "08:00", 0)).toBe(true);
    expect(inTimeWindow("22:00", "08:00", 7 * 60 + 59)).toBe(true);
    expect(inTimeWindow("22:00", "08:00", 8 * 60)).toBe(false);
    expect(inTimeWindow("22:00", "08:00", 12 * 60)).toBe(false);
  });
});

describe("shouldNotify", () => {
  const base: AppSettings = {
    engine_mode: "local",
    library_path: "",
    naming_template: "{title}.{ext}",
    lan_enabled: false,
    lan_port: 8787,
    download_quality: "1080",
    prefer_mp4: true,
    notify_download_complete: true,
    quiet_hours_enabled: false,
    quiet_hours_from: "22:00",
    quiet_hours_to: "08:00",
  };

  it("respects event toggles", () => {
    expect(shouldNotify({ ...base, notify_download_complete: false }, "download_complete")).toBe(
      false,
    );
    expect(shouldNotify(base, "download_complete")).toBe(true);
  });

  it("suppresses during quiet hours", () => {
    const quiet: AppSettings = { ...base, quiet_hours_enabled: true };
    expect(shouldNotify(quiet, "download_complete", 23 * 60)).toBe(false);
    expect(shouldNotify(quiet, "download_complete", 12 * 60)).toBe(true);
  });

  it("allows when quiet hours disabled even at night", () => {
    expect(shouldNotify(base, "watch_import", 23 * 60)).toBe(true);
  });
});
