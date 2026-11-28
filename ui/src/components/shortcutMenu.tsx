import Link from "lucide-solid/icons/link";
import type { ShortcutPlace } from "../bindings/ShortcutPlace";
import { translator } from "../i18n";
import { shortcutsText } from "../i18n/shortcuts";
import { createShortcuts } from "../lib/api";
import { errorMessage } from "../lib/errors";
import { store } from "../lib/store";
import type { MenuEntry } from "./Menu";

const tr = translator(shortcutsText);

/** Creates shortcuts for these games and says how it went in a toast. */
export async function createShortcutsFor(ids: string[], places: ShortcutPlace[]): Promise<void> {
  try {
    const result = await createShortcuts(ids, places);
    if (result.created > 0) {
      store.pushToast(
        "info",
        result.created === 1 ? tr("created") : tr("createdMany", { count: result.created }),
      );
    }
    if (result.failed.length > 0) {
      store.pushToast("error", tr("failed", { reasons: result.failed.slice(0, 3).join("; ") }));
    }
  } catch (failure) {
    store.pushToast("error", errorMessage(failure));
  }
}

/** The "Create shortcut" submenu of a game menu (or of several selected games). */
export function shortcutMenuEntry(ids: () => string[]): MenuEntry {
  const make = (places: ShortcutPlace[]) => () => void createShortcutsFor(ids(), places);
  return {
    kind: "submenu",
    label: ids().length > 1 ? tr("createShortcuts") : tr("createShortcut"),
    icon: () => <Link class="icon" aria-hidden="true" />,
    entries: () => [
      { label: tr("desktop"), onSelect: make(["desktop"]) },
      { label: tr("startMenu"), onSelect: make(["startMenu"]) },
      { label: tr("both"), onSelect: make(["desktop", "startMenu"]) },
    ],
  };
}
