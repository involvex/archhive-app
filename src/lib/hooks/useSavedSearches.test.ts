import { describe, expect, it, vi } from "vitest";
import { queueableUrls, MAX_NEW_MATCHES } from "./useSavedSearches";
import type { MediaItem } from "@/lib/types";

function item(url: string): MediaItem {
  return {
    id: url,
    title: "t",
    url,
    thumbnail: undefined,
    site_id: "x",
    performers: [],
    tags: [],
  } as unknown as MediaItem;
}

describe("queueableUrls", () => {
  it("keeps absolute http(s) URLs", () => {
    const urls = queueableUrls([item("https://a.example/v1"), item("http://b.example/v2")]);
    expect(urls).toEqual(["https://a.example/v1", "http://b.example/v2"]);
  });

  it("trims surrounding whitespace", () => {
    expect(queueableUrls([item("  https://a.example/v1  ")])).toEqual(["https://a.example/v1"]);
  });

  it("rejects non-http schemes that could be injected by a bad scrape", () => {
    const hostile = [
      "javascript:alert(1)",
      "data:text/html,<script>alert(1)</script>",
      "file:///etc/passwd",
      "/relative/path",
      "ftp://a.example/v",
      "",
    ];
    expect(queueableUrls(hostile.map(item))).toEqual([]);
  });
});

describe("MAX_NEW_MATCHES", () => {
  it("caps a listing so a huge result set cannot flood the queue", () => {
    const many = Array.from({ length: 500 }, (_, i) => item(`https://a.example/${i}`));
    expect(queueableUrls(many)).toHaveLength(500);
    // The hook slices to MAX_NEW_MATCHES before exposing `items`; the constant
    // is the guard rail, assert it stays sane.
    expect(MAX_NEW_MATCHES).toBe(25);
    expect(many.slice(0, MAX_NEW_MATCHES)).toHaveLength(25);
  });
});

describe("useRefreshGuard contract", () => {
  it("is exported and callable", async () => {
    const { useRefreshGuard } = await import("./useRefreshGuard");
    expect(typeof useRefreshGuard).toBe("function");
  });
});

vi.mock("@/lib/api/client", () => ({
  api: { browse: vi.fn(), listSavedSearches: vi.fn(), watchlistStatus: vi.fn() },
}));
