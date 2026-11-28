import { describe, expect, it } from "vitest";
import { hasVaultPinConfigured, shouldEngageAppLock } from "@/lib/appLock";

describe("appLock (R11)", () => {
  it("locks on cold start when enabled with a PIN", () => {
    expect(
      shouldEngageAppLock({
        enabled: true,
        pinConfigured: true,
        hiddenAt: null,
        now: 1_000,
        timeoutMins: 5,
      }),
    ).toBe(true);
  });

  it("skips when disabled or PIN missing", () => {
    expect(
      shouldEngageAppLock({
        enabled: false,
        pinConfigured: true,
        hiddenAt: null,
        now: 1_000,
        timeoutMins: 5,
      }),
    ).toBe(false);
    expect(
      shouldEngageAppLock({
        enabled: true,
        pinConfigured: false,
        hiddenAt: null,
        now: 1_000,
        timeoutMins: 5,
      }),
    ).toBe(false);
  });

  it("locks after timeout and immediately when timeout is 0", () => {
    const hiddenAt = 1_000;
    expect(
      shouldEngageAppLock({
        enabled: true,
        pinConfigured: true,
        hiddenAt,
        now: hiddenAt + 5 * 60_000,
        timeoutMins: 5,
      }),
    ).toBe(true);
    expect(
      shouldEngageAppLock({
        enabled: true,
        pinConfigured: true,
        hiddenAt,
        now: hiddenAt + 60_000,
        timeoutMins: 5,
      }),
    ).toBe(false);
    expect(
      shouldEngageAppLock({
        enabled: true,
        pinConfigured: true,
        hiddenAt,
        now: hiddenAt + 1,
        timeoutMins: 0,
      }),
    ).toBe(true);
  });

  it("detects configured PIN hashes", () => {
    expect(hasVaultPinConfigured(undefined)).toBe(false);
    expect(hasVaultPinConfigured("")).toBe(false);
    expect(hasVaultPinConfigured("ab:cd")).toBe(true);
  });
});
