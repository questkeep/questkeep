import type { AddGameSection } from "../bindings/AddGameSection";
import type { PriceSite } from "../bindings/PriceSite";
import type { ProviderInfo } from "../bindings/ProviderInfo";

/** The submenus of the "Add game" menu, in menu order. */
export type AddMenuSection = "add" | "compare" | AddGameSection;
export const ADD_MENU_SECTIONS: readonly AddMenuSection[] = ["add", "compare", "buy", "activate"];

/** The quick-access id of the manual add ("Add a game…"). */
export const MANUAL_ID = "add:manual";

/** What choosing an entry does. */
export type AddMenuAction =
  | { kind: "manual" }
  | {
      kind: "option";
      providerId: string;
      providerName: string;
      optionId: string;
      /** It opens a web page in the browser (not the store's launcher). */
      inBrowser: boolean;
    }
  | { kind: "site"; url: string };

/** One entry of the "Add game" menu. */
export interface AddMenuItem {
  /** Stable id, as saved in the quick-access list. */
  id: string;
  section: AddMenuSection;
  /** The label in its submenu: an option's label, a store or site name. */
  label: string;
  /** Small text after the label in the Add submenu (the store's name). */
  hint?: string;
  action: AddMenuAction;
}

export interface AddMenuGroup {
  section: AddMenuSection;
  items: AddMenuItem[];
}

/**
 * Every entry of the "Add game" menu, by submenu: the manual add and the stores' ways to add
 * games, the price sites, the stores' shops and key redemptions. Stores in the given order; only
 * submenus with entries.
 */
export function addMenuGroups(
  providers: ProviderInfo[],
  sites: PriceSite[],
  manualLabel: string,
): AddMenuGroup[] {
  const groups: Record<AddMenuSection, AddMenuItem[]> = {
    add: [{ id: MANUAL_ID, section: "add", label: manualLabel, action: { kind: "manual" } }],
    compare: sites.map((site) => ({
      id: `compare:${site.url}`,
      section: "compare",
      label: site.name,
      action: { kind: "site", url: site.url },
    })),
    buy: [],
    activate: [],
  };
  for (const provider of providers) {
    for (const option of provider.addGameOptions ?? []) {
      const section = option.section ?? "add";
      groups[section].push({
        id: `${section}:${provider.id}/${option.id}`,
        section,
        label: option.label,
        hint: section === "add" ? provider.name : undefined,
        action: {
          kind: "option",
          providerId: provider.id,
          providerName: provider.name,
          optionId: option.id,
          inBrowser: option.opens.kind === "browser",
        },
      });
    }
  }
  return ADD_MENU_SECTIONS.map((section) => ({ section, items: groups[section] })).filter(
    (group) => group.items.length > 0,
  );
}

/**
 * An entry's label in the quick-access part of the menu: the submenu's name in front ("Buy from
 * Steam"); the manual add has its own (`manualQuickLabel`, "Add a game…").
 */
export function quickLabel(
  item: AddMenuItem,
  sectionLabel: (section: AddMenuSection, name: string) => string,
  manualQuickLabel: string,
): string {
  return item.id === MANUAL_ID ? manualQuickLabel : sectionLabel(item.section, item.label);
}

/** The saved quick-access ids that exist now, as entries, in the saved order. */
export function quickItems(ids: string[], groups: AddMenuGroup[]): AddMenuItem[] {
  const byId = new Map(groups.flatMap((g) => g.items).map((item) => [item.id, item]));
  const seen = new Set<string>();
  const items: AddMenuItem[] = [];
  for (const id of ids) {
    const item = byId.get(id);
    if (item && !seen.has(id)) {
      seen.add(id);
      items.push(item);
    }
  }
  return items;
}

/** The list with `id` added at the end (`on`) or taken out. */
export function toggleQuick(ids: string[], id: string, on: boolean): string[] {
  const without = ids.filter((i) => i !== id);
  return on ? [...without, id] : without;
}

/**
 * The saved list after the shown entries were put in `order`: ids that aren't shown (their store
 * is gone) keep their places; the shown ones fill their slots in the new order.
 */
export function reorderQuick(ids: string[], order: string[]): string[] {
  const shown = new Set(order);
  const queue = [...order];
  return ids.map((id) => (shown.has(id) ? (queue.shift() ?? id) : id));
}

/** `order` with `id` moved to where `target` is (dragging over it). */
export function moveQuickTo(order: string[], id: string, target: string): string[] {
  const from = order.indexOf(id);
  const to = order.indexOf(target);
  if (from < 0 || to < 0 || from === to) return order;
  const next = [...order];
  next.splice(from, 1);
  next.splice(to, 0, id);
  return next;
}

/** `order` with `id` moved `by` places (Alt+Up/Down), kept within the list. */
export function moveQuickBy(order: string[], id: string, by: -1 | 1): string[] {
  const from = order.indexOf(id);
  const target = order[from + by];
  return from < 0 || target === undefined ? order : moveQuickTo(order, id, target);
}
