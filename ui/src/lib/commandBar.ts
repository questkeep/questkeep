import type { JSX } from "solid-js";
import type { Game } from "../bindings/Game";
import type { MenuEntry } from "../components/Menu";

/** How many results the command bar shows. */
export const COMMAND_RESULTS = 7;

/**
 * Orders name matches for the command bar: visible games before hidden ones, then names where
 * the match starts earlier, then by sorting name. Keeps at most {@link COMMAND_RESULTS}.
 */
export function rankMatches(games: readonly Game[], search: string): Game[] {
  const needle = search.trim().toLowerCase();
  const position = (game: Game) => {
    const at = game.name.toLowerCase().indexOf(needle);
    return at < 0 ? Number.MAX_SAFE_INTEGER : at;
  };
  return games
    .toSorted(
      (a, b) =>
        Number(a.hidden) - Number(b.hidden) ||
        position(a) - position(b) ||
        a.sortingName.localeCompare(b.sortingName),
    )
    .slice(0, COMMAND_RESULTS);
}

/** How many commands the command bar shows under the games at most. */
export const COMMAND_LIMIT = 3;

/** Joins a menu path in a command's label ("Sync › Steam"). */
export const PATH_SEPARATOR = " › ";

/**
 * Something the command bar can do besides opening a game: go to a page, an entry of the
 * Questkeep menu, an action script of the selected game.
 */
export interface AppCommand {
  /** Stable within one list (for keys and tests). */
  id: string;
  label: string;
  /** More words it is found by, not shown. */
  keywords?: string;
  /** Small text at the end of the row (a shortcut). */
  hint?: string;
  icon?: () => JSX.Element;
  run: () => void;
}

/**
 * The actions of a menu as commands: submenus become part of the label ("Sync › Steam", but
 * "Sync all" where the entry already starts with the submenu's name),
 * separators, headings and disabled entries are left out.
 */
export function flattenMenu(
  entries: readonly MenuEntry[],
  path: readonly string[] = [],
  idPrefix = "menu",
): AppCommand[] {
  return entries.flatMap((entry, index): AppCommand[] => {
    const id = `${idPrefix}.${index}`;
    if (entry.kind === "separator" || entry.kind === "heading") return [];
    if (entry.kind === "submenu") return flattenMenu(entry.entries(), [...path, entry.label], id);
    if (entry.disabled) return [];
    // "Sync all" in the Sync submenu stays "Sync all", not "Sync › Sync all".
    const parent = path.at(-1)?.toLowerCase();
    const repeats = parent !== undefined && entry.label.toLowerCase().startsWith(parent);
    return [
      {
        id,
        label: [...(repeats ? path.slice(0, -1) : path), entry.label].join(PATH_SEPARATOR),
        hint: entry.hint,
        icon: entry.icon,
        run: entry.onSelect,
      },
    ];
  });
}

/** Where a label starts a word: its start, or after anything but a letter or digit. */
function startsWord(text: string, needle: string): boolean {
  for (let at = text.indexOf(needle); at >= 0; at = text.indexOf(needle, at + 1)) {
    if (at === 0 || !/[\p{L}\p{N}]/u.test(text[at - 1] ?? "")) return true;
  }
  return false;
}

/**
 * The commands that match the search, best first, at most `limit` (none for an empty search).
 * Every word of the search must appear in the label or the keywords. Labels that start with the
 * search come first, then those where it starts a word, then any other place in the label, then
 * matches through the keywords or scattered words; shorter labels first within each.
 */
export function matchCommands(
  commands: readonly AppCommand[],
  search: string,
  limit = COMMAND_LIMIT,
): AppCommand[] {
  const needle = search.trim().toLowerCase();
  if (needle === "") return [];
  const words = needle.split(/\s+/);
  const score = (command: AppCommand) => {
    const label = command.label.toLowerCase();
    if (label.startsWith(needle)) return 0;
    if (startsWord(label, needle)) return 1;
    if (label.includes(needle)) return 2;
    return 3;
  };
  return commands
    .filter((command) => {
      const text = `${command.label} ${command.keywords ?? ""}`.toLowerCase();
      return words.every((word) => text.includes(word));
    })
    .map((command) => ({ command, score: score(command) }))
    .toSorted((a, b) => a.score - b.score || a.command.label.length - b.command.label.length)
    .slice(0, limit)
    .map(({ command }) => command);
}

/** A name split around the first case-insensitive match, for highlighting. */
export function splitMatch(
  name: string,
  search: string,
): { before: string; match: string; after: string } {
  const needle = search.trim();
  const at = needle === "" ? -1 : name.toLowerCase().indexOf(needle.toLowerCase());
  if (at < 0) return { before: name, match: "", after: "" };
  return {
    before: name.slice(0, at),
    match: name.slice(at, at + needle.length),
    after: name.slice(at + needle.length),
  };
}
