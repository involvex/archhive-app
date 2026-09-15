import { describe, it, expect, vi, afterEach } from "vitest";
import { vibrateTick } from "@/lib/haptics";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("vibrateTick (Q39)", () => {
  it("returns false when navigator.vibrate is unavailable", () => {
    vi.stubGlobal("navigator", {});
    expect(vibrateTick()).toBe(false);
  });

  it("requests the default 10ms tick and returns the result", () => {
    const vibrate = vi.fn().mockReturnValue(true);
    vi.stubGlobal("navigator", { vibrate });
    expect(vibrateTick()).toBe(true);
    expect(vibrate).toHaveBeenCalledWith(10);
  });

  it("forwards custom patterns", () => {
    const vibrate = vi.fn().mockReturnValue(true);
    vi.stubGlobal("navigator", { vibrate });
    expect(vibrateTick([20, 30, 20])).toBe(true);
    expect(vibrate).toHaveBeenCalledWith([20, 30, 20]);
  });

  it("returns false when vibrate throws", () => {
    const vibrate = vi.fn().mockImplementation(() => {
      throw new Error("denied");
    });
    vi.stubGlobal("navigator", { vibrate });
    expect(vibrateTick()).toBe(false);
  });
});
