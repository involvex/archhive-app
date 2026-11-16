import { describe, expect, it } from "vitest";
import { visibleSettingsTabs } from "./capabilities";
import type { RuntimeCapabilities } from "../runtime";

const base: RuntimeCapabilities = {
  localIpc: true,
  lanServer: false,
  libraryScanLocal: true,
  libraryScanRemote: false,
  showBrowserBanner: false,
  lanDiscovery: true,
  desktopShell: false,
  engineModes: ["local", "remote_lan"],
};

describe("visibleSettingsTabs", () => {
  it("hides the Desktop tab on phones even with a local backend", () => {
    expect(visibleSettingsTabs(base)).not.toContain("desktop");
  });

  it("shows the Desktop tab in the desktop app", () => {
    expect(visibleSettingsTabs({ ...base, desktopShell: true, lanServer: true })).toContain(
      "desktop",
    );
  });

  it("keeps the LAN tab visible on phones (hosting gated by lanServer)", () => {
    expect(visibleSettingsTabs(base)).toContain("lan");
  });
});
