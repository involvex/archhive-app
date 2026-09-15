import { describe, it, expect, vi } from "vitest";
import { isWakeLockSupported, requestScreenWakeLock } from "@/lib/hooks/useWakeLock";

function stubWakeLock(request: (type: string) => Promise<{ release: () => Promise<void> }>) {
  Object.defineProperty(window.navigator, "wakeLock", {
    value: { request },
    configurable: true,
  });
}

describe("isWakeLockSupported (Q33)", () => {
  it("is false without the API", () => {
    expect(isWakeLockSupported()).toBe(false);
  });

  it("is true with the API", () => {
    stubWakeLock(() => Promise.resolve({ release: () => Promise.resolve() }));
    expect(isWakeLockSupported()).toBe(true);
    // @ts-expect-error test cleanup removes the stubbed API
    delete window.navigator.wakeLock;
  });
});

describe("requestScreenWakeLock (Q33)", () => {
  it("requests a screen lock and returns the sentinel", async () => {
    const sentinel = { release: vi.fn().mockResolvedValue(undefined) };
    const request = vi.fn().mockResolvedValue(sentinel);
    await expect(requestScreenWakeLock(request)).resolves.toBe(sentinel);
    expect(request).toHaveBeenCalledWith("screen");
  });

  it("returns null when the request is denied", async () => {
    const request = vi.fn().mockRejectedValue(new Error("denied"));
    await expect(requestScreenWakeLock(request)).resolves.toBeNull();
  });
});
