import { describe, expect, it } from "vitest";
import {
  cookieDomainMatchesHostKey,
  cookieHostLabel,
  hostCookieKey,
  XPOMELO_COOKIE_HOSTS,
} from "./cookieHosts";

describe("per-domain cookie keys", () => {
  it("matches the Rust key rule", () => {
    expect(hostCookieKey("https://www.SpankBang.com/8va69/video/x")).toBe("host-spankbang-com");
    expect(hostCookieKey("4kporn.xxx")).toBe("host-4kporn-xxx");
    expect(hostCookieKey("hoes.tube/")).toBe("host-hoes-tube");
    expect(hostCookieKey("localhost")).toBeNull();
    expect(hostCookieKey("not a domain")).toBeNull();
  });

  it("matches cookie domains incl. subdomains", () => {
    expect(cookieDomainMatchesHostKey(".spankbang.com", "host-spankbang-com")).toBe(true);
    expect(cookieDomainMatchesHostKey("m.spankbang.com", "host-spankbang-com")).toBe(true);
    expect(cookieDomainMatchesHostKey(".pornhub.com", "host-spankbang-com")).toBe(false);
  });

  it("labels known sources and keeps keys unique", () => {
    expect(cookieHostLabel("host-fapcat-com")).toBe("FapCat (fapcat.com)");
    expect(cookieHostLabel("host-example-org")).toBe("host-example-org");
    const keys = XPOMELO_COOKIE_HOSTS.map((h) => hostCookieKey(h.domain));
    expect(new Set(keys).size).toBe(keys.length);
  });
});
