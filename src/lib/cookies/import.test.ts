import { describe, expect, it } from "vitest";
import {
  cookieHeaderToNetscape,
  guessCookieSiteFromText,
  jsonToNetscape,
  looksLikeCookieExport,
  parseCookieHeader,
} from "./import";

describe("parseCookieHeader", () => {
  it("parses a simple cookie header into cookie objects", () => {
    const cookies = parseCookieHeader("sess=abc123; uid=42", "thothub.tv");
    expect(cookies).toHaveLength(2);
    expect(cookies[0]).toMatchObject({
      name: "sess",
      value: "abc123",
      domain: "thothub.tv",
      path: "/",
      hostOnly: true,
    });
    expect(cookies[1]).toMatchObject({ name: "uid", value: "42" });
  });

  it("keeps '=' characters inside the value", () => {
    const cookies = parseCookieHeader("token=a=b=c", "x.com");
    expect(cookies).toHaveLength(1);
    expect(cookies[0].value).toBe("a=b=c");
  });

  it("skips malformed pairs and blank segments", () => {
    const cookies = parseCookieHeader("novalue; a=1;  ; =orphan", "x.com");
    expect(cookies.map((c) => c.name)).toEqual(["a"]);
  });

  it("marks leading-dot domains as non-host-only", () => {
    const cookies = parseCookieHeader("a=1", ".example.com");
    expect(cookies[0].hostOnly).toBe(false);
  });
});

describe("cookieHeaderToNetscape", () => {
  it("generates a Netscape file for the harvested header", () => {
    const netscape = cookieHeaderToNetscape("sess=abc; uid=1", "thothub.tv");
    const lines = netscape.split("\n");
    expect(lines[0]).toBe("# Netscape HTTP Cookie File");
    const cookieLines = lines.filter((line) => line && !line.startsWith("#"));
    expect(cookieLines).toHaveLength(2);
    expect(cookieLines[0]).toContain("\tsess\tabc");
    expect(cookieLines[0]).toContain(".thothub.tv");
  });

  it("throws when nothing was captured", () => {
    expect(() => cookieHeaderToNetscape("   ", "thothub.tv")).toThrow();
  });
});

describe("looksLikeCookieExport", () => {
  it("accepts a Cookie-Editor style JSON array", () => {
    const raw = JSON.stringify([{ name: "sess", value: "abc", domain: ".thothub.tv", path: "/" }]);
    expect(looksLikeCookieExport(raw)).toBe(true);
  });

  it("accepts a { cookies: [...] } wrapper", () => {
    const raw = JSON.stringify({ cookies: [{ name: "a", value: "1", domain: ".x.com" }] });
    expect(looksLikeCookieExport(raw)).toBe(true);
  });

  it("rejects ordinary shared URLs", () => {
    expect(looksLikeCookieExport("https://thothub.tv/video/123")).toBe(false);
    expect(looksLikeCookieExport("thothub.tv/video")).toBe(false);
  });

  it("rejects JSON that is not a cookie list", () => {
    expect(looksLikeCookieExport(JSON.stringify([{ a: 1 }, { b: 2 }]))).toBe(false);
    expect(looksLikeCookieExport(JSON.stringify({ hello: "world" }))).toBe(false);
    expect(looksLikeCookieExport("{ not json")).toBe(false);
  });
});

describe("guessCookieSiteFromText", () => {
  it("maps known cookie domains to a site id", () => {
    const raw = JSON.stringify([{ name: "sess", value: "abc", domain: ".redgifs.com", path: "/" }]);
    expect(guessCookieSiteFromText(raw)).toBe("redgifs");
  });

  it("returns null when no known domain matches", () => {
    const raw = JSON.stringify([{ name: "a", value: "1", domain: ".unknown.example" }]);
    expect(guessCookieSiteFromText(raw)).toBeNull();
  });

  it("returns null for non-cookie text", () => {
    expect(guessCookieSiteFromText("https://redgifs.com/watch/abc")).toBeNull();
  });

  it("detects the X/Twitter jar", () => {
    const raw = JSON.stringify([{ name: "auth_token", value: "abc", domain: ".x.com", path: "/" }]);
    expect(guessCookieSiteFromText(raw)).toBe("twitter");
  });

  it("detects ThotHub on its real domain", () => {
    const raw = JSON.stringify([
      { name: "sess", value: "abc", domain: "thethothub.com", path: "/" },
    ]);
    expect(guessCookieSiteFromText(raw)).toBe("thothub");
  });
});

describe("jsonToNetscape", () => {
  it("marks host-only cookies without subdomains", () => {
    const netscape = jsonToNetscape([
      { name: "a", value: "1", domain: "thothub.tv", path: "/", hostOnly: true },
    ]);
    const line = netscape.split("\n").find((l) => l.includes("\ta\t"));
    expect(line).toContain(".thothub.tv\tFALSE\t/\tFALSE\t0\ta\t1");
  });
});
