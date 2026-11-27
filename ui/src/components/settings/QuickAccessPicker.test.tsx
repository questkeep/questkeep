import { fireEvent, render, screen, within } from "@solidjs/testing-library";
import { createSignal } from "solid-js";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ProviderInfo } from "../../bindings/ProviderInfo";
import * as api from "../../lib/api";
import { store } from "../../lib/store";
import { QuickAccessPicker } from "./QuickAccessPicker";

vi.mock("../../lib/api");
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (p: string) => p,
  invoke: vi.fn<() => void>(),
}));

const web = (url: string) => ({ kind: "browser", url }) as const;

const steam: ProviderInfo = {
  id: "steam",
  name: "Steam",
  icon: "<svg></svg>",
  canScan: true,
  canScanOwned: false,
  canUninstall: false,
  launcherPages: false,
  launcherClose: null,
  installed: true,
  syncable: true,
  addGameOptions: [
    { id: "shop", label: "Steam", section: "buy", opens: web("https://s.example/") },
    { id: "redeem", label: "Steam", section: "activate", opens: web("https://s.example/k") },
  ],
  settings: { options: [], credentials: null, notice: null },
};

/** The picker with its saved list kept in a signal, as the settings page does. */
function renderPicker(initial: string[]) {
  const [ids, setIds] = createSignal(initial);
  const onChange = vi.fn<(next: string[]) => void>((next) => setIds(next));
  render(() => <QuickAccessPicker ids={ids()} onChange={onChange} labelledBy="x" />);
  return { ids, onChange };
}

const chosenNames = () =>
  [...document.querySelectorAll(".quick-access .quick-launch__item .quick-launch__name")].map(
    (n) => n.textContent,
  );

beforeEach(async () => {
  store.reset();
  vi.mocked(api.listProviders).mockResolvedValue([steam]);
  await store.loadProviders();
});

describe("QuickAccessPicker", () => {
  it("shows the count and lists the chosen entries with their menu labels", async () => {
    renderPicker(["activate:steam/redeem", "gone:x", "add:manual"]);
    const button = screen.getByRole("button", { name: /Entries: 2/ });
    await fireEvent.click(button);
    expect(screen.getByRole("dialog", { name: "Quick access" })).toBeInTheDocument();
    expect(chosenNames()).toEqual(["Activate on Steam", "Add a game…"]);
  });

  it("checks and unchecks entries in the tree", async () => {
    const { ids } = renderPicker(["add:manual"]);
    await fireEvent.click(screen.getByRole("button", { name: /Entries: 1/ }));
    const tree = screen.getByRole("tree", { name: "All entries" });
    const sections = within(tree)
      .getAllByRole("treeitem")
      .filter((i) => i.getAttribute("aria-expanded") !== null);
    expect(sections.map((s) => s.querySelector(".quick-access__row")?.textContent)).toEqual([
      "Add",
      "Buy from",
      "Activate on",
    ]);
    // Only the submenu holding a chosen entry starts open.
    expect(sections.map((s) => s.getAttribute("aria-expanded"))).toEqual([
      "true",
      "false",
      "false",
    ]);
    await fireEvent.click(sections[1]?.querySelector(".quick-access__row") as HTMLElement);
    const buy = within(sections[1] as HTMLElement).getByRole("treeitem");
    expect(buy).toHaveAttribute("aria-checked", "false");
    await fireEvent.click(buy);
    expect(ids()).toEqual(["add:manual", "buy:steam/shop"]);
    expect(chosenNames()).toEqual(["Add a game…", "Buy from Steam"]);
    await fireEvent.keyDown(buy, { key: " " });
    expect(ids()).toEqual(["add:manual"]);
  });

  it("removes and reorders chosen entries, keeping hidden ones in place", async () => {
    const { ids } = renderPicker(["add:manual", "gone:x", "buy:steam/shop"]);
    await fireEvent.click(screen.getByRole("button", { name: /Entries: 2/ }));
    const row = document.querySelector<HTMLElement>('[data-quick-id="buy:steam/shop"]');
    if (!row) throw new Error("no row");
    await fireEvent.keyDown(row, { key: "ArrowUp", altKey: true });
    expect(ids()).toEqual(["buy:steam/shop", "gone:x", "add:manual"]);
    await fireEvent.click(screen.getByRole("button", { name: "Remove Add a game…" }));
    expect(ids()).toEqual(["buy:steam/shop", "gone:x"]);
  });

  it("moves through the tree with the arrows and folds sections", async () => {
    renderPicker([]);
    await fireEvent.click(screen.getByRole("button", { name: /No entries/ }));
    expect(screen.getByText("Check entries below to add them to the menu.")).toBeInTheDocument();
    const tree = screen.getByRole("tree");
    const first = tree.querySelector<HTMLElement>('[data-row="s:add"]');
    if (!first) throw new Error("no section");
    expect(first).toHaveAttribute("tabindex", "0");
    expect(first).toHaveAttribute("aria-expanded", "false");
    await fireEvent.keyDown(first, { key: "ArrowDown" });
    expect(document.activeElement).toBe(tree.querySelector('[data-row="s:buy"]'));
    await fireEvent.keyDown(first, { key: "ArrowRight" });
    expect(first).toHaveAttribute("aria-expanded", "true");
    await fireEvent.keyDown(first, { key: "ArrowRight" });
    expect(document.activeElement).toBe(tree.querySelector('[data-row="i:add:manual"]'));
    await fireEvent.keyDown(document.activeElement as HTMLElement, { key: "ArrowLeft" });
    expect(document.activeElement).toBe(first);
    await fireEvent.keyDown(first, { key: "ArrowLeft" });
    expect(first).toHaveAttribute("aria-expanded", "false");
  });
});
