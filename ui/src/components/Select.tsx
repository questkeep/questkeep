import Check from "lucide-solid/icons/check";
import ChevronDown from "lucide-solid/icons/chevron-down";
import {
  createEffect,
  createMemo,
  createSignal,
  createUniqueId,
  For,
  Match,
  Show,
  Switch,
  type JSX,
} from "solid-js";
import {
  moveSelectIndex,
  selectKeyAction,
  typeAheadActive,
  typeAheadIndex,
  typeAheadNext,
  type TypeAhead,
} from "../lib/selectKeys";
import { Popover, type PopoverAnchor } from "./Popover";

export type SelectValue = string | number;

export interface SelectOption<T extends SelectValue> {
  value: T;
  label: string;
  icon?: () => JSX.Element;
  disabled?: boolean;
}

/** Options under a heading (like `<optgroup>`). */
export interface SelectGroup<T extends SelectValue> {
  group: string;
  options: SelectOption<T>[];
}

export type SelectItem<T extends SelectValue> = SelectOption<T> | SelectGroup<T>;

export interface SelectProps<T extends SelectValue> {
  value: T;
  options: SelectItem<T>[];
  onChange: (value: T) => void;
  id?: string;
  /** Classes of the trigger button (`input`, a width class…). */
  class?: string;
  disabled?: boolean;
  /** Shown when the value is none of the options. */
  placeholder?: string;
  "aria-label"?: string;
  "aria-labelledby"?: string;
  "aria-describedby"?: string;
}

const isGroup = <T extends SelectValue>(item: SelectItem<T>): item is SelectGroup<T> =>
  "group" in item;

type Row<T extends SelectValue> =
  | { kind: "option"; option: SelectOption<T>; index: number }
  | { kind: "group"; label: string; options: { option: SelectOption<T>; index: number }[] };

/**
 * A dropdown in the app's own look, in place of a native `<select>`: a button showing the chosen
 * option that opens a list in a Popover styled like the menus. Focus stays on the button (a
 * select-only combobox: the highlighted option is its `aria-activedescendant`). Closed, keys only
 * open the list; the value changes when an option is chosen.
 */
export function Select<T extends SelectValue>(props: SelectProps<T>) {
  const listId = createUniqueId();
  const [open, setOpen] = createSignal(false);
  const [active, setActive] = createSignal(-1);
  const [anchor, setAnchor] = createSignal<{ at: PopoverAnchor; width: number } | null>(null);
  let trigger: HTMLButtonElement | undefined;
  let search: TypeAhead | null = null;

  const flat = createMemo(() =>
    props.options.flatMap((item) => (isGroup(item) ? item.options : [item])),
  );
  const rows = createMemo(() => {
    let index = 0;
    return props.options.map((item): Row<T> => {
      if (!isGroup(item)) return { kind: "option", option: item, index: index++ };
      return {
        kind: "group",
        label: item.group,
        options: item.options.map((option) => ({ option, index: index++ })),
      };
    });
  });
  const disabledList = () => flat().map((option) => option.disabled === true);
  const selectedIndex = () => flat().findIndex((option) => option.value === props.value);
  const selected = () => flat()[selectedIndex()];
  const optionId = (index: number) => `${listId}-${index}`;

  function show(at?: "first" | "last") {
    if (props.disabled || !trigger) return;
    const current = selectedIndex();
    const start =
      at === undefined && current >= 0 && !disabledList()[current]
        ? current
        : moveSelectIndex(at ?? "first", -1, disabledList());
    setActive(start);
    const rect = trigger.getBoundingClientRect();
    setAnchor({ at: { rect, align: "below" }, width: rect.width });
    setOpen(true);
  }

  function close(returnFocus: boolean) {
    setOpen(false);
    search = null;
    if (returnFocus) trigger?.focus();
  }

  function choose(index: number) {
    const option = flat()[index];
    close(true);
    if (!option || option.disabled) return;
    if (option.value !== props.value) props.onChange(option.value);
  }

  // Keeps the highlighted option in view in a long list.
  createEffect(() => {
    if (!open() || active() < 0) return;
    const element = document.getElementById(optionId(active()));
    element?.scrollIntoView?.({ block: "nearest" });
  });

  function onKeyDown(event: KeyboardEvent) {
    const now = Date.now();
    const action = selectKeyAction(event, open(), typeAheadActive(search, now));
    if (!action) return;
    switch (action.kind) {
      case "open":
        show();
        break;
      case "openAt":
        show(action.at);
        break;
      case "move":
        setActive(moveSelectIndex(action.to, active(), disabledList()));
        break;
      case "choose":
        if (active() >= 0) choose(active());
        else close(true);
        break;
      case "close":
        // Tab closes and lets the browser move focus on.
        if (!action.returnFocus) {
          close(false);
          return;
        }
        close(true);
        event.stopPropagation(); // Escape closes only the list, not a dialog around it
        break;
      case "type": {
        search = typeAheadNext(search, action.char, now);
        const labels = flat().map((option) => option.label);
        const from = open() ? active() : selectedIndex();
        const found = typeAheadIndex(labels, disabledList(), from, search.text);
        if (!open()) show();
        if (found >= 0) setActive(found);
        break;
      }
    }
    event.preventDefault();
  }

  /** The list's name: the trigger's own label. */
  const listLabel = () =>
    props["aria-label"] ??
    (props["aria-labelledby"]
      ? document.getElementById(props["aria-labelledby"])?.textContent
      : trigger?.labels?.[0]?.textContent) ??
    undefined;

  const option = (entry: { option: SelectOption<T>; index: number }) => (
    <div
      id={optionId(entry.index)}
      role="option"
      class="menu__item select__option"
      classList={{ "menu__item--active": active() === entry.index }}
      aria-selected={entry.option.value === props.value}
      aria-disabled={entry.option.disabled === true ? true : undefined}
      onPointerMove={() => {
        if (!entry.option.disabled) setActive(entry.index);
      }}
      onClick={() => {
        if (!entry.option.disabled) choose(entry.index);
      }}
    >
      <span class="menu__icon">
        <Show when={entry.option.value === props.value}>
          <Check class="icon menu__check" aria-hidden="true" />
        </Show>
      </span>
      <Show when={entry.option.icon}>
        {(icon) => <span class="menu__icon select__option-icon">{icon()()}</span>}
      </Show>
      <span class="menu__label">{entry.option.label}</span>
    </div>
  );

  return (
    <>
      <button
        ref={(el) => (trigger = el)}
        type="button"
        id={props.id}
        class={`select ${props.class ?? ""}`}
        role="combobox"
        aria-haspopup="listbox"
        aria-expanded={open()}
        aria-controls={open() ? listId : undefined}
        aria-activedescendant={open() && active() >= 0 ? optionId(active()) : undefined}
        aria-label={props["aria-label"]}
        aria-labelledby={props["aria-labelledby"]}
        aria-describedby={props["aria-describedby"]}
        disabled={props.disabled}
        onClick={() => (open() ? close(true) : show())}
        onKeyDown={onKeyDown}
        // Space was handled on keydown; its keyup must not click the button again.
        onKeyUp={(event) => {
          if (event.key === " ") event.preventDefault();
        }}
      >
        <Show when={selected()?.icon}>
          {(icon) => <span class="select__icon">{icon()()}</span>}
        </Show>
        <span class="select__value">{selected()?.label ?? props.placeholder ?? ""}</span>
        <ChevronDown class="icon select__chevron" aria-hidden="true" />
      </button>
      <Show when={open() && anchor()}>
        {(at) => (
          <Popover
            anchor={at().at}
            class="menu select__list"
            role="listbox"
            label={listLabel()}
            inside={() => [trigger]}
            onClose={close}
            ref={(panel) => {
              panel.id = listId;
              panel.style.setProperty("--qk-select-trigger-width", `${at().width}px`);
              // Clicking an option must not take focus off the trigger.
              panel.addEventListener("mousedown", (event) => event.preventDefault());
            }}
          >
            <For each={rows()}>
              {(row) => (
                <Switch>
                  <Match when={row.kind === "option" && row}>{(entry) => option(entry())}</Match>
                  <Match when={row.kind === "group" && row}>
                    {(group) => (
                      <div role="group" aria-label={group().label} class="select__group">
                        <div class="menu__heading" aria-hidden="true">
                          {group().label}
                        </div>
                        <For each={group().options}>{(entry) => option(entry)}</For>
                      </div>
                    )}
                  </Match>
                </Switch>
              )}
            </For>
          </Popover>
        )}
      </Show>
    </>
  );
}
