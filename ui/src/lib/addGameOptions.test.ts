import { describe, expect, it } from "vitest";
import type { AddGameOption } from "../bindings/AddGameOption";
import type { ProviderInfo } from "../bindings/ProviderInfo";
import { addGameMenuItems, nextMenuIndex } from "./addGameOptions";

const provider = (id: string, options: AddGameOption[]): ProviderInfo => ({
  id,
  name: id.toUpperCase(),
  icon: "",
  canScan: true,
  canScanOwned: false,
  canUninstall: false,
  launcherPages: false,
  launcherClose: null,
  shopUrl: null,
  installed: true,
  syncable: true,
  addGameOptions: options,
  settings: { options: [], credentials: null, notice: null },
});

describe("addGameMenuItems", () => {
  it("flattens the options of every provider in order", () => {
    const items = addGameMenuItems([
      provider("steam", []),
      provider("prism", [
        { id: "new-instance", label: "New Minecraft instance", opens: { kind: "provider" } },
      ]),
      provider("x", [
        { id: "a", label: "A", opens: { kind: "provider" } },
        { id: "b", label: "B", opens: { kind: "browser", url: "https://b.example" } },
      ]),
    ]);
    expect(items.map((i) => `${i.providerId}/${i.optionId}`)).toEqual([
      "prism/new-instance",
      "x/a",
      "x/b",
    ]);
    expect(items[0]).toMatchObject({
      providerName: "PRISM",
      label: "New Minecraft instance",
      inBrowser: false,
    });
    expect(items[2]?.inBrowser).toBe(true);
  });

  it("is empty without options", () => {
    expect(addGameMenuItems([provider("steam", [])])).toEqual([]);
  });
});

describe("nextMenuIndex", () => {
  it("wraps around and jumps to the ends", () => {
    expect(nextMenuIndex("ArrowDown", 2, 3)).toBe(0);
    expect(nextMenuIndex("ArrowUp", 0, 3)).toBe(2);
    expect(nextMenuIndex("Home", 2, 3)).toBe(0);
    expect(nextMenuIndex("End", 0, 3)).toBe(2);
  });

  it("ignores other keys and empty menus", () => {
    expect(nextMenuIndex("a", 0, 3)).toBeNull();
    expect(nextMenuIndex("ArrowDown", 0, 0)).toBeNull();
  });
});
