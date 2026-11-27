import ChevronDown from "lucide-solid/icons/chevron-down";
import ChevronRight from "lucide-solid/icons/chevron-right";
import GripVertical from "lucide-solid/icons/grip-vertical";
import X from "lucide-solid/icons/x";
import { createSignal, For, Show } from "solid-js";
import { translator } from "../../i18n";
import { settingsGeneral } from "../../i18n/settingsGeneral";
import {
  moveQuickBy,
  moveQuickTo,
  quickItems,
  reorderQuick,
  toggleQuick,
  type AddMenuSection,
} from "../../lib/addMenu";
import { AddMenuIcon, currentAddMenuGroups, quickItemLabel, sectionName } from "../AddMenuParts";
import { Popover, type PopoverAnchor } from "../Popover";

const tr = translator(settingsGeneral);

/** One focusable row of the tree: a section or an entry in it. */
type TreeRow = { kind: "section"; section: AddMenuSection } | { kind: "item"; id: string };
const rowKey = (row: TreeRow) => (row.kind === "section" ? `s:${row.section}` : `i:${row.id}`);

/**
 * The quick-access entries of the Add game menu: a dropdown showing their count, whose popup has
 * the chosen entries in menu order on top (drag the grip or Alt+Up/Down to reorder, ✕ to remove)
 * and every entry below as a tree of submenus with checkboxes (arrows move, Right/Left open and
 * close a submenu, Space checks).
 */
export function QuickAccessPicker(props: {
  ids: string[];
  onChange: (ids: string[]) => void;
  labelledBy: string;
}) {
  const [anchor, setAnchor] = createSignal<PopoverAnchor | null>(null);
  const [collapsed, setCollapsed] = createSignal<Set<AddMenuSection>>(new Set());
  /** The shown order while an entry is dragged (saved when it is dropped). */
  const [draft, setDraft] = createSignal<string[] | null>(null);
  const [dragged, setDragged] = createSignal<string | null>(null);
  const [active, setActive] = createSignal<string | null>(null);
  let button: HTMLButtonElement | undefined;
  let tree: HTMLUListElement | undefined;
  let list: HTMLUListElement | undefined;

  const groups = currentAddMenuGroups;
  const chosen = () => quickItems(props.ids, groups());
  const order = () => draft() ?? chosen().map((item) => item.id);
  const shown = () => {
    const byId = new Map(chosen().map((item) => [item.id, item]));
    return order().flatMap((id) => byId.get(id) ?? []);
  };
  const isChosen = (id: string) => chosen().some((item) => item.id === id);

  const count = () =>
    chosen().length === 0
      ? tr("quickAccessNone")
      : tr("quickAccessCount", { count: chosen().length });

  /** The tree's rows as shown (collapsed sections without their entries), top to bottom. */
  const rows = (): TreeRow[] => {
    const all: TreeRow[] = [];
    for (const group of groups()) {
      all.push({ kind: "section", section: group.section });
      if (collapsed().has(group.section)) continue;
      for (const item of group.items) all.push({ kind: "item", id: item.id });
    }
    return all;
  };

  function open() {
    if (anchor()) {
      setAnchor(null);
      return;
    }
    // Only the submenus that hold a chosen entry start open, so the popup stays short.
    const holding = new Set(chosen().map((item) => item.section));
    setCollapsed(
      new Set(
        groups()
          .map((g) => g.section)
          .filter((s) => !holding.has(s)),
      ),
    );
    const rect = button?.getBoundingClientRect();
    if (rect) setAnchor({ rect, align: "below" });
    setActive(rows()[0] ? rowKey(rows()[0] as TreeRow) : null);
  }

  function close(returnFocus: boolean) {
    setAnchor(null);
    if (returnFocus) button?.focus();
  }

  const save = (ids: string[]) => props.onChange(ids);
  const toggle = (id: string) => save(toggleQuick(props.ids, id, !isChosen(id)));
  const setOrder = (next: string[]) => save(reorderQuick(props.ids, next));

  function focusRow(key: string) {
    setActive(key);
    tree?.querySelector<HTMLElement>(`[data-row="${CSS.escape(key)}"]`)?.focus();
  }

  function setSectionOpen(section: AddMenuSection, isOpen: boolean) {
    setCollapsed((all) => {
      const next = new Set(all);
      if (isOpen) next.delete(section);
      else next.add(section);
      return next;
    });
  }

  function onTreeKey(event: KeyboardEvent, row: TreeRow) {
    const all = rows();
    const at = all.findIndex((r) => rowKey(r) === rowKey(row));
    const go = (target: TreeRow | undefined) => {
      event.preventDefault();
      if (target) focusRow(rowKey(target));
    };
    switch (event.key) {
      case "ArrowDown":
        return go(all[at + 1]);
      case "ArrowUp":
        return go(all[at - 1]);
      case "Home":
        return go(all[0]);
      case "End":
        return go(all[all.length - 1]);
      case "ArrowRight":
        if (row.kind === "section") {
          event.preventDefault();
          if (collapsed().has(row.section)) setSectionOpen(row.section, true);
          else go(all[at + 1]);
        }
        return;
      case "ArrowLeft": {
        event.preventDefault();
        if (row.kind === "section") setSectionOpen(row.section, false);
        else go(all.slice(0, at).findLast((r) => r.kind === "section"));
        return;
      }
      case " ":
      case "Enter":
        event.preventDefault();
        if (row.kind === "item") toggle(row.id);
        else setSectionOpen(row.section, collapsed().has(row.section));
        return;
    }
  }

  function moveChosen(id: string, by: -1 | 1) {
    setOrder(moveQuickBy(order(), id, by));
    queueMicrotask(() =>
      list?.querySelector<HTMLElement>(`[data-quick-id="${CSS.escape(id)}"]`)?.focus(),
    );
  }

  function drop() {
    const next = draft();
    setDragged(null);
    setDraft(null);
    if (next) setOrder(next);
  }

  return (
    <>
      <button
        ref={(el) => (button = el)}
        type="button"
        class="select input settings-row__input"
        aria-haspopup="dialog"
        aria-expanded={anchor() !== null}
        aria-labelledby={`${props.labelledBy} quick-access-count`}
        onClick={open}
      >
        <span class="select__value" id="quick-access-count">
          {count()}
        </span>
        <ChevronDown class="icon select__chevron" aria-hidden="true" />
      </button>
      <Show when={anchor()}>
        {(at) => (
          <Popover
            anchor={at()}
            class="quick-access"
            role="dialog"
            label={tr("quickAccess")}
            inside={() => [button]}
            onClose={close}
          >
            <div class="menu__heading" id="quick-access-chosen">
              {tr("quickAccessChosen")}
            </div>
            <Show
              when={shown().length > 0}
              fallback={<p class="field__hint quick-access__empty">{tr("quickAccessEmpty")}</p>}
            >
              <ul
                class="quick-launch"
                ref={(el) => (list = el)}
                aria-labelledby="quick-access-chosen"
              >
                <For each={shown()}>
                  {(item) => (
                    <li
                      class="quick-launch__item"
                      classList={{ "quick-launch__item--dragging": dragged() === item.id }}
                      data-quick-id={item.id}
                      tabIndex={0}
                      draggable={true}
                      onDragStart={(event) => {
                        setDragged(item.id);
                        event.dataTransfer?.setData("text/plain", item.id);
                      }}
                      onDragEnd={drop}
                      onDragOver={(event) => {
                        const from = dragged();
                        if (from === null) return;
                        event.preventDefault();
                        if (from !== item.id) setDraft(moveQuickTo(order(), from, item.id));
                      }}
                      onDrop={(event) => event.preventDefault()}
                      onKeyDown={(event) => {
                        if (
                          !event.altKey ||
                          (event.key !== "ArrowUp" && event.key !== "ArrowDown")
                        ) {
                          return;
                        }
                        event.preventDefault();
                        moveChosen(item.id, event.key === "ArrowUp" ? -1 : 1);
                      }}
                    >
                      <GripVertical class="icon quick-launch__grip" aria-hidden="true" />
                      <span class="quick-launch__icon" aria-hidden="true">
                        <AddMenuIcon item={item} />
                      </span>
                      <span class="quick-launch__name">{quickItemLabel(item)}</span>
                      <button
                        type="button"
                        class="icon-btn icon-btn--small"
                        aria-label={tr("quickAccessRemove", { name: quickItemLabel(item) })}
                        data-tooltip={tr("quickAccessRemove", { name: quickItemLabel(item) })}
                        onClick={() => toggle(item.id)}
                      >
                        <X class="icon" aria-hidden="true" />
                      </button>
                    </li>
                  )}
                </For>
              </ul>
            </Show>
            <div class="menu__separator" role="separator" />
            <ul
              class="quick-access__tree"
              role="tree"
              aria-label={tr("quickAccessAll")}
              ref={(el) => (tree = el)}
            >
              <For each={groups()}>
                {(group) => {
                  const sectionRow: TreeRow = { kind: "section", section: group.section };
                  const isOpen = () => !collapsed().has(group.section);
                  return (
                    <li
                      role="treeitem"
                      aria-expanded={isOpen()}
                      class="quick-access__section"
                      data-row={rowKey(sectionRow)}
                      tabIndex={active() === rowKey(sectionRow) ? 0 : -1}
                      onFocus={(event) => {
                        if (event.target === event.currentTarget) setActive(rowKey(sectionRow));
                      }}
                      onKeyDown={(event) => {
                        if (event.target === event.currentTarget) onTreeKey(event, sectionRow);
                      }}
                    >
                      <span
                        class="quick-access__row"
                        onClick={() => setSectionOpen(group.section, !isOpen())}
                      >
                        <Show
                          when={isOpen()}
                          fallback={
                            <ChevronRight class="icon quick-access__chevron" aria-hidden="true" />
                          }
                        >
                          <ChevronDown class="icon quick-access__chevron" aria-hidden="true" />
                        </Show>
                        {sectionName(group.section)}
                      </span>
                      <Show when={isOpen()}>
                        <ul role="group">
                          <For each={group.items}>
                            {(item) => {
                              const row: TreeRow = { kind: "item", id: item.id };
                              return (
                                <li
                                  role="treeitem"
                                  aria-checked={isChosen(item.id)}
                                  class="quick-access__row quick-access__item"
                                  data-row={rowKey(row)}
                                  tabIndex={active() === rowKey(row) ? 0 : -1}
                                  onFocus={() => setActive(rowKey(row))}
                                  onKeyDown={(event) => {
                                    event.stopPropagation();
                                    onTreeKey(event, row);
                                  }}
                                  onClick={() => {
                                    focusRow(rowKey(row));
                                    toggle(item.id);
                                  }}
                                >
                                  <input
                                    type="checkbox"
                                    tabIndex={-1}
                                    aria-hidden="true"
                                    checked={isChosen(item.id)}
                                    onClick={(event) => event.preventDefault()}
                                  />
                                  <span class="quick-launch__icon" aria-hidden="true">
                                    <AddMenuIcon item={item} />
                                  </span>
                                  <span class="quick-launch__name">{item.label}</span>
                                  <Show when={item.hint}>
                                    <span class="menu__hint">{item.hint}</span>
                                  </Show>
                                </li>
                              );
                            }}
                          </For>
                        </ul>
                      </Show>
                    </li>
                  );
                }}
              </For>
            </ul>
          </Popover>
        )}
      </Show>
    </>
  );
}
