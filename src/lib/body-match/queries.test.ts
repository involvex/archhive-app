import { describe, expect, it } from "vitest";
import { buildBodyMatchLinks } from "./queries";

describe("buildBodyMatchLinks", () => {
  it("builds six encoded links for DD + brunette", () => {
    const links = buildBodyMatchLinks({ cup: "DD", hair: "brunette" });
    expect(links).toHaveLength(6);
    const sites = links.map((l) => l.site);
    expect(sites).toEqual([
      "pornhub",
      "xvideos",
      "spankbang",
      "boobpedia",
      "babepedia",
      "google-iafd",
    ]);

    const pornhub = links[0];
    expect(pornhub.url).toBe("https://www.pornhub.com/video/search?search=brunette%20DD%20cup");

    const google = links.find((l) => l.site === "google-iafd");
    expect(google?.query).toContain("site:iafd.com");
    expect(google?.query).toContain("34DD");
    expect(google?.url.startsWith("https://www.google.com/search?q=")).toBe(true);
  });

  it("omits hair filter for any", () => {
    const links = buildBodyMatchLinks({ cup: "big-tits", hair: "any" });
    expect(links[0].query).toBe("big tits");
  });

  it("uses plus-slugs for spankbang", () => {
    const links = buildBodyMatchLinks({ cup: "DD", hair: "brunette" });
    const sb = links.find((l) => l.site === "spankbang");
    expect(sb?.url).toBe("https://spankbang.com/s/brunette+DD+cup/");
  });
});
