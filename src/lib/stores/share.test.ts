import { beforeEach, describe, expect, it } from "vitest";
import { useShareStore } from "./share";

beforeEach(() => {
  useShareStore.getState().clearPendingShare();
});

describe("useShareStore", () => {
  it("ignores blank shares", () => {
    useShareStore.getState().setPendingShare("   \n  ");
    expect(useShareStore.getState().pendingShare).toBeNull();
  });

  it("stores a single shared URL", () => {
    useShareStore.getState().setPendingShare("https://example.com/watch/123");
    expect(useShareStore.getState().pendingShare).toBe("https://example.com/watch/123");
  });

  it("appends follow-up shares instead of dropping them", () => {
    const store = useShareStore.getState();
    store.setPendingShare("https://example.com/watch/1");
    useShareStore.getState().setPendingShare("https://example.com/watch/2");
    expect(useShareStore.getState().pendingShare).toBe(
      "https://example.com/watch/1\nhttps://example.com/watch/2",
    );
  });

  it("takePendingShare returns text once and clears", () => {
    useShareStore.getState().setPendingShare("https://example.com/watch/1");
    expect(useShareStore.getState().takePendingShare()).toBe("https://example.com/watch/1");
    expect(useShareStore.getState().pendingShare).toBeNull();
    expect(useShareStore.getState().takePendingShare()).toBeNull();
  });
});
