/** Keyboard logic of the Select dropdown (components/Select.tsx), kept pure for tests. */

/** What a key does to a Select. Closed, no key changes the value: keys only open the list. */
export type SelectKeyAction =
  /** Open the list on the selected option (or the first one). */
  | { kind: "open" }
  /** Open the list on the first / last option. */
  | { kind: "openAt"; at: "first" | "last" }
  /** Move the highlight (list open). */
  | { kind: "move"; to: "next" | "previous" | "first" | "last" | "pageDown" | "pageUp" }
  /** Choose the highlighted option and close. */
  | { kind: "choose" }
  /** Close without choosing; `returnFocus` is false for Tab (focus moves on). */
  | { kind: "close"; returnFocus: boolean }
  /** A printable character for type-ahead. */
  | { kind: "type"; char: string };

export interface SelectKey {
  key: string;
  altKey?: boolean;
  ctrlKey?: boolean;
  metaKey?: boolean;
}

/** How many options PageUp/PageDown skip. */
export const SELECT_PAGE = 10;

/**
 * The action of a key on a Select trigger (focus stays on the trigger while the list is open).
 * `typing` is true while a type-ahead search is going on: Space then extends it instead of choosing.
 */
export function selectKeyAction(
  event: SelectKey,
  open: boolean,
  typing = false,
): SelectKeyAction | null {
  const { key } = event;
  if (event.ctrlKey || event.metaKey) return null;
  if (key.length === 1 && key !== " " && !event.altKey) return { kind: "type", char: key };
  if (!open) {
    switch (key) {
      case "Enter":
      case " ":
      case "ArrowDown":
      case "ArrowUp":
        return { kind: "open" };
      case "Home":
        return { kind: "openAt", at: "first" };
      case "End":
        return { kind: "openAt", at: "last" };
      default:
        return null;
    }
  }
  if (event.altKey && key === "ArrowUp") return { kind: "choose" };
  switch (key) {
    case "ArrowDown":
      return event.altKey ? null : { kind: "move", to: "next" };
    case "ArrowUp":
      return { kind: "move", to: "previous" };
    case "Home":
      return { kind: "move", to: "first" };
    case "End":
      return { kind: "move", to: "last" };
    case "PageDown":
      return { kind: "move", to: "pageDown" };
    case "PageUp":
      return { kind: "move", to: "pageUp" };
    case " ":
      return typing ? { kind: "type", char: " " } : { kind: "choose" };
    case "Enter":
      return { kind: "choose" };
    case "Escape":
      return { kind: "close", returnFocus: true };
    case "Tab":
      return { kind: "close", returnFocus: false };
    default:
      return null;
  }
}

/** The first enabled index at or after `from` going in `step` direction, or -1. */
function enabledFrom(disabled: readonly boolean[], from: number, step: 1 | -1): number {
  for (let i = from; i >= 0 && i < disabled.length; i += step) if (!disabled[i]) return i;
  return -1;
}

/**
 * The option to highlight after a move from `current` (-1: none). Disabled options are skipped;
 * the highlight stops at the ends instead of wrapping (like a native list).
 */
export function moveSelectIndex(
  to: Extract<SelectKeyAction, { kind: "move" }>["to"],
  current: number,
  disabled: readonly boolean[],
): number {
  const count = disabled.length;
  const first = enabledFrom(disabled, 0, 1);
  const last = enabledFrom(disabled, count - 1, -1);
  if (first < 0) return -1;
  const keep = (index: number) => (index < 0 ? current : index);
  switch (to) {
    case "first":
      return first;
    case "last":
      return last;
    case "next":
      return current < 0 ? first : keep(enabledFrom(disabled, current + 1, 1));
    case "previous":
      return current < 0 ? last : keep(enabledFrom(disabled, current - 1, -1));
    case "pageDown": {
      const found = enabledFrom(
        disabled,
        Math.min(count - 1, Math.max(current, 0) + SELECT_PAGE),
        -1,
      );
      return found > current ? found : current;
    }
    case "pageUp": {
      if (current < 0) return first;
      const found = enabledFrom(disabled, Math.max(0, current - SELECT_PAGE), 1);
      return found >= 0 && found < current ? found : current;
    }
  }
}

/** A type-ahead search: the typed text and when the last character came. */
export interface TypeAhead {
  text: string;
  at: number;
}

/** How long (ms) after the last character a new one starts a new search. */
export const TYPE_AHEAD_RESET = 500;

/** The search after typing `char` at `now`. */
export function typeAheadNext(previous: TypeAhead | null, char: string, now: number): TypeAhead {
  const fresh = previous === null || now - previous.at > TYPE_AHEAD_RESET;
  return { text: (fresh ? "" : previous.text) + char.toLowerCase(), at: now };
}

/** Whether a type-ahead search is still going on at `now`. */
export function typeAheadActive(search: TypeAhead | null, now: number): boolean {
  return search !== null && search.text !== "" && now - search.at <= TYPE_AHEAD_RESET;
}

/**
 * The option a type-ahead search lands on, or -1. Several presses of the same letter cycle
 * through the options starting with it; longer text finds the first label starting with it,
 * looking from the current option on (it stays there while it still matches).
 */
export function typeAheadIndex(
  labels: readonly string[],
  disabled: readonly boolean[],
  current: number,
  text: string,
): number {
  if (text === "") return -1;
  const lower = labels.map((label) => label.trim().toLowerCase());
  const repeated = [...text].every((c) => c === text[0]);
  const needle = repeated ? text.charAt(0) : text;
  // One letter (or the same letter again): the next match after the current option.
  // Longer text: the current option still counts.
  const start = repeated ? current + 1 : Math.max(current, 0);
  const count = labels.length;
  for (let n = 0; n < count; n++) {
    const index = (start + n) % count;
    if (!disabled[index] && lower[index]?.startsWith(needle)) return index;
  }
  return -1;
}
