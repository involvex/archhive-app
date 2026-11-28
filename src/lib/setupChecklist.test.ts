import { beforeEach, describe, expect, it } from "vitest";
import type { AppSettings } from "@/lib/types";
import {
  clearSetupChecklistDismissed,
  dismissSetupChecklist,
  evaluateSetupChecklist,
  isSetupChecklistDismissed,
  shouldShowSetupChecklist,
} from "@/lib/setupChecklist";

function baseSettings(partial: Partial<AppSettings> = {}): AppSettings {
  return {
    engine_mode: "local",
    library_path: "D:/ArcHive",
    lan_enabled: false,
    lan_port: 8787,
    remote_host: "",
    ...partial,
  } as AppSettings;
}

describe("setupChecklist (R07)", () => {
  beforeEach(() => {
    clearSetupChecklistDismissed();
    localStorage.clear();
  });

  it("marks library/engine done for a local desktop with a path", () => {
    const items = evaluateSetupChecklist({
      settings: baseSettings({ lan_enabled: true }),
      cookieSiteCount: 2,
      canHostLan: true,
    });
    expect(items.every((i) => i.done)).toBe(true);
  });

  it("requires remote_host when engine is remote_lan", () => {
    const incomplete = evaluateSetupChecklist({
      settings: baseSettings({ engine_mode: "remote_lan", remote_host: "" }),
      cookieSiteCount: 0,
      canHostLan: false,
    });
    expect(incomplete.find((i) => i.id === "engine")?.done).toBe(false);
    expect(incomplete.find((i) => i.id === "lan")?.done).toBe(false);
    expect(incomplete.find((i) => i.id === "cookies")?.done).toBe(false);

    const complete = evaluateSetupChecklist({
      settings: baseSettings({
        engine_mode: "remote_lan",
        remote_host: "http://192.168.1.2:8787",
      }),
      cookieSiteCount: 1,
      canHostLan: false,
    });
    expect(complete.find((i) => i.id === "engine")?.done).toBe(true);
    expect(complete.find((i) => i.id === "lan")?.done).toBe(true);
  });

  it("persists dismiss", () => {
    expect(isSetupChecklistDismissed()).toBe(false);
    dismissSetupChecklist();
    expect(isSetupChecklistDismissed()).toBe(true);
  });

  it("hides when wizard incomplete, dismissed, or all done", () => {
    const pending = evaluateSetupChecklist({
      settings: baseSettings({ lan_enabled: false }),
      cookieSiteCount: 0,
      canHostLan: true,
    });
    expect(shouldShowSetupChecklist(true, pending, false)).toBe(true);
    expect(shouldShowSetupChecklist(false, pending, false)).toBe(false);
    expect(shouldShowSetupChecklist(true, pending, true)).toBe(false);

    const done = evaluateSetupChecklist({
      settings: baseSettings({ lan_enabled: true }),
      cookieSiteCount: 1,
      canHostLan: true,
    });
    expect(shouldShowSetupChecklist(true, done, false)).toBe(false);
  });
});
