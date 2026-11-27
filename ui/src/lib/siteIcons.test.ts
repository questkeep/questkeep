import { describe, expect, it } from "vitest";
import { siteIconKey, siteIconUrl, siteIconUrls } from "./siteIcons";

describe("site icons", () => {
  it("keys sites by their host name", () => {
    expect(siteIconKey("https://WWW.MyDealz.de/deals?x=1")).toBe("www.mydealz.de");
    expect(siteIconKey("https://example.com:8443/")).toBe("example.com");
    expect(siteIconKey("not an address")).toBeNull();
  });

  it("finds a site's icon with its version", () => {
    const urls = siteIconUrls(
      [{ host: "gg.deals", path: "C:\\data\\images\\sites\\gg.deals.png", version: 42 }],
      (path) => `asset://${path}`,
    );
    expect(siteIconUrl(urls, "https://gg.deals/")).toBe(
      "asset://C:\\data\\images\\sites\\gg.deals.png?v=42",
    );
    expect(siteIconUrl(urls, "https://steamdb.info/")).toBeNull();
    expect(siteIconUrl(urls, "garbage")).toBeNull();
  });
});
