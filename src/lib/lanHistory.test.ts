import { describe, it, expect, beforeEach } from "vitest";
import { loadLanHistory, recordLanHost, clearLanHistory } from "@/lib/lanHistory";

describe("lanHistory (Q35)", () => {
  beforeEach(() => localStorage.clear());

  it("starts empty", () => {
    expect(loadLanHistory()).toEqual([]);
  });

  it("records a host with token flag", () => {
    const next = recordLanHost("http://192.168.178.69:8787", { hasToken: true });
    expect(next).toHaveLength(1);
    expect(next[0]?.url).toBe("http://192.168.178.69:8787");
    expect(next[0]?.hasToken).toBe(true);
  });

  it("dedupes by URL and keeps most-recent-first, capped at 3", () => {
    recordLanHost("http://a:8787");
    recordLanHost("http://b:8787");
    recordLanHost("http://c:8787");
    recordLanHost("http://a:8787"); // re-use moves to front
    const next = recordLanHost("http://d:8787");
    expect(next.map((e) => e.url)).toEqual(["http://d:8787", "http://a:8787", "http://c:8787"]);
  });

  it("normalizes trailing slashes when deduping", () => {
    recordLanHost("http://a:8787/");
    expect(loadLanHistory()).toHaveLength(1);
    recordLanHost("http://a:8787");
    expect(loadLanHistory()).toHaveLength(1);
  });

  it("ignores blank URLs", () => {
    recordLanHost("   ");
    expect(loadLanHistory()).toEqual([]);
  });

  it("clears", () => {
    recordLanHost("http://a:8787");
    clearLanHistory();
    expect(loadLanHistory()).toEqual([]);
  });

  it("falls back to empty on corrupt storage", () => {
    localStorage.setItem("archhive.lanHistory", "not-json");
    expect(loadLanHistory()).toEqual([]);
  });
});
