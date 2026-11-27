import { describe, expect, it } from "vitest";
import type { AddGameOption } from "../bindings/AddGameOption";
import type { ProviderInfo } from "../bindings/ProviderInfo";
import {
  addMenuGroups,
  MANUAL_ID,
  moveQuickBy,
  moveQuickTo,
  quickItems,
  quickLabel,
  reorderQuick,
  toggleQuick,
} from "./addMenu";

const provider = (id: string, options: AddGameOption[]): ProviderInfo => ({
  id,
  name: id.toUpperCase(),
  icon: "",
  canScan: true,
  canScanOwned: false,
  canUninstall: false,
  launcherPages: false,
  launcherClose: null,
  installed: true,
  syncable: true,
  addGameOptions: options,
  settings: { options: [], credentials: null, notice: null },
});

const web = (url: string) => ({ kind: "browser", url }) as const;

const PROVIDERS = [
  provider("steam", [
    { id: "shop", label: "Steam", section: "buy", opens: { kind: "uri", uri: "steam://store" } },
    { id: "redeem", label: "Steam", section: "activate", opens: web("https://s.example/key") },
  ]),
  provider("prism", [
    { id: "new", label: "New Minecraft instance", section: "add", opens: { kind: "provider" } },
  ]),
  provider("gog", [
    { id: "shop", label: "GOG", section: "buy", opens: web("https://gog.example") },
  ]),
];

const SITES = [{ name: "GG.deals", url: "https://gg.deals/" }];

const ids = (groups: ReturnType<typeof addMenuGroups>) =>
  groups.map((g) => [g.section, g.items.map((i) => i.id)]);

describe("addMenuGroups", () => {
  it("sorts every entry into its submenu, stores in order", () => {
    const groups = addMenuGroups(PROVIDERS, SITES, "Manual game…");
    expect(ids(groups)).toEqual([
      ["add", [MANUAL_ID, "add:prism/new"]],
      ["compare", ["compare:https://gg.deals/"]],
      ["buy", ["buy:steam/shop", "buy:gog/shop"]],
      ["activate", ["activate:steam/redeem"]],
    ]);
    const prism = groups[0]?.items[1];
    expect(prism).toMatchObject({ label: "New Minecraft instance", hint: "PRISM" });
    expect(prism?.action).toMatchObject({ kind: "option", providerId: "prism", inBrowser: false });
    expect(groups[3]?.items[0]?.action).toMatchObject({ inBrowser: true });
    expect(groups[2]?.items[0]?.hint).toBeUndefined();
  });

  it("leaves out empty submenus", () => {
    expect(ids(addMenuGroups([provider("x", [])], [], "Manual"))).toEqual([["add", [MANUAL_ID]]]);
  });
});

const label = (section: string, name: string) => `${section} ${name}`;

describe("quick access", () => {
  const groups = addMenuGroups(PROVIDERS, SITES, "Manual game…");

  it("resolves saved ids in order, skipping gone and repeated ones", () => {
    const items = quickItems(
      ["activate:steam/redeem", "buy:epic/shop", MANUAL_ID, "activate:steam/redeem"],
      groups,
    );
    expect(items.map((i) => i.id)).toEqual(["activate:steam/redeem", MANUAL_ID]);
    expect(items.map((i) => quickLabel(i, label, "Add a game…"))).toEqual([
      "activate Steam",
      "Add a game…",
    ]);
  });

  it("adds at the end and removes", () => {
    expect(toggleQuick([MANUAL_ID], "buy:gog/shop", true)).toEqual([MANUAL_ID, "buy:gog/shop"]);
    expect(toggleQuick(["a", "b"], "a", true)).toEqual(["b", "a"]);
    expect(toggleQuick(["a", "b"], "a", false)).toEqual(["b"]);
  });

  it("reorders the shown entries and keeps hidden ones in place", () => {
    expect(reorderQuick(["a", "gone", "b", "c"], ["c", "a", "b"])).toEqual(["c", "gone", "a", "b"]);
    expect(moveQuickTo(["a", "b", "c"], "a", "c")).toEqual(["b", "c", "a"]);
    expect(moveQuickTo(["a", "b", "c"], "c", "a")).toEqual(["c", "a", "b"]);
    expect(moveQuickBy(["a", "b", "c"], "b", -1)).toEqual(["b", "a", "c"]);
    expect(moveQuickBy(["a", "b", "c"], "c", 1)).toEqual(["a", "b", "c"]);
  });
});
