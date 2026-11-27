import { fireEvent, render } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as api from "../lib/api";
import { store } from "../lib/store";
import { SiteIcon } from "./SiteIcon";

vi.mock("../lib/api");
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (p: string) => `asset://${p}`,
  invoke: vi.fn<() => void>(),
}));

beforeEach(() => {
  vi.clearAllMocks();
  store.reset();
});

const icon = (version: number) => ({
  host: "gg.deals",
  path: "/data/images/sites/gg.deals.png",
  version,
});

describe("SiteIcon", () => {
  it("shows the globe while a site has no icon", async () => {
    vi.mocked(api.priceSiteIcons).mockResolvedValue([icon(1)]);
    await store.loadSiteIcons();
    const { container } = render(() => <SiteIcon url="https://steamdb.info/" />);
    expect(container.querySelector("img")).toBeNull();
    expect(container.querySelector("svg")).not.toBeNull();
  });

  it("shows the cached icon and falls back to the globe when it can't be loaded", async () => {
    vi.mocked(api.priceSiteIcons).mockResolvedValue([icon(1)]);
    await store.loadSiteIcons();
    const { container } = render(() => <SiteIcon url="https://gg.deals/" />);
    const img = container.querySelector("img");
    expect(img?.getAttribute("src")).toBe("asset:///data/images/sites/gg.deals.png?v=1");
    expect(img?.getAttribute("alt")).toBe("");

    fireEvent.error(img as HTMLImageElement);
    expect(container.querySelector("img")).toBeNull();
    expect(container.querySelector("svg")).not.toBeNull();

    // A refreshed icon gets another chance.
    vi.mocked(api.priceSiteIcons).mockResolvedValue([icon(2)]);
    await store.loadSiteIcons();
    expect(container.querySelector("img")?.getAttribute("src")).toBe(
      "asset:///data/images/sites/gg.deals.png?v=2",
    );
  });

  it("follows the icons when the core reports new ones", async () => {
    vi.mocked(api.priceSiteIcons).mockResolvedValue([]);
    await store.loadSiteIcons();
    const { container } = render(() => <SiteIcon url="https://gg.deals/" />);
    expect(container.querySelector("img")).toBeNull();

    vi.mocked(api.priceSiteIcons).mockResolvedValue([icon(3)]);
    store.handleCoreEvent({ type: "siteIconsChanged" });
    await vi.waitFor(() => expect(container.querySelector("img")).not.toBeNull());
  });
});
