import { describe, expect, it, vi } from "vitest";
import { openExternal } from "./external";

describe("openExternal", () => {
  it("falls back to window.open outside Tauri", async () => {
    const spy = vi.fn();
    vi.stubGlobal("open", spy);
    await openExternal("https://example.com/search?q=test");
    expect(spy).toHaveBeenCalledWith("https://example.com/search?q=test", "_blank", "noopener");
    vi.unstubAllGlobals();
  });
});
