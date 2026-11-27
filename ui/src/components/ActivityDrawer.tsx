import ArrowRight from "lucide-solid/icons/arrow-right";
import X from "lucide-solid/icons/x";
import { createMemo, createSignal, createUniqueId, onCleanup, onMount } from "solid-js";
import type { ActivityCursor } from "../bindings/ActivityCursor";
import type { ActivityItem } from "../bindings/ActivityItem";
import type { Game } from "../bindings/Game";
import { translator } from "../i18n";
import { activity as dict } from "../i18n/activity";
import {
  DEFAULT_LIBRARY_FILTER,
  isFiltered,
  itemAt,
  itemKey,
  libraryQuery,
  type LibraryFilter,
} from "../lib/activity";
import * as api from "../lib/api";
import { store } from "../lib/store";
import { LibraryFilters } from "./ActivityFilters";
import { ItemRow, PagedLog } from "./ActivityItems";

const tr = translator(dict);

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

/**
 * A game's whole log in a panel from the right (from the game page, its "…" menu and its
 * right-click menu). Focus stays inside until it closes and then goes back where it was.
 */
export function ActivityDrawer(props: { game: Game; onClose: () => void }) {
  const titleId = createUniqueId();
  const [filter, setFilter] = createSignal<LibraryFilter>({
    ...DEFAULT_LIBRARY_FILTER,
    gameId: props.game.id,
  });
  let panel: HTMLElement | undefined;
  const previouslyFocused = document.activeElement;

  const load = createMemo(() => {
    const current = filter();
    return (before: ActivityCursor | null) =>
      api.activityPage(libraryQuery(current, new Date(), before));
  });
  // Only changes of this game (or of the whole library) reload it.
  let relevant = 0;
  const refresh = createMemo(() => {
    const change = store.activityChange();
    if (change.gameIds.length === 0 || change.gameIds.includes(props.game.id))
      relevant = change.seq;
    return relevant;
  });

  onMount(() => {
    panel?.querySelector<HTMLElement>("[data-autofocus]")?.focus();
  });
  onCleanup(() => {
    if (previouslyFocused instanceof HTMLElement && previouslyFocused.isConnected) {
      previouslyFocused.focus();
    }
  });

  function onKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.stopPropagation();
      props.onClose();
      return;
    }
    if (event.key !== "Tab" || !panel) return;
    const items = Array.from(panel.querySelectorAll<HTMLElement>(FOCUSABLE));
    const first = items[0];
    const last = items.at(-1);
    if (!first || !last) return;
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  function openPage() {
    const { who, what, sessions } = filter();
    store.showActivity({
      tab: "library",
      library: { gameId: props.game.id, who, what, sessions },
      system: {},
      gameName: props.game.name,
    });
  }

  return (
    <div class="alog-drawer-wrap">
      <div class="alog-scrim" onMouseDown={() => props.onClose()} />
      <aside
        ref={(el) => (panel = el)}
        class="alog-drawer"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        onKeyDown={onKeyDown}
      >
        <header class="alog-drawer__head">
          <div class="alog-drawer__titles">
            <h2 id={titleId} class="alog-drawer__title">
              {tr("title")}
            </h2>
            <p class="alog-drawer__sub">{props.game.name}</p>
          </div>
          <button
            type="button"
            class="icon-btn"
            data-autofocus
            aria-label={tr("close")}
            data-tooltip={tr("close")}
            onClick={() => props.onClose()}
          >
            <X class="icon" aria-hidden="true" />
          </button>
        </header>
        <LibraryFilters
          compact
          filter={filter()}
          onChange={(patch) => setFilter((f) => ({ ...f, ...patch }))}
        />
        <div class="alog-drawer__scroll">
          <PagedLog<ActivityItem>
            load={load()}
            refresh={refresh}
            key={itemKey}
            at={itemAt}
            label={tr("activityOf", { name: props.game.name })}
            empty={
              isFiltered({ ...filter(), gameId: null }) ? tr("emptyFiltered") : tr("noActivity")
            }
            row={(item) => (
              // Entries of games combined into this one name their game.
              <ItemRow
                item={item}
                showGame={item.type === "entry" && item.gameId !== props.game.id}
              />
            )}
          />
        </div>
        <footer class="alog-drawer__foot">
          <button type="button" class="link-btn alog-link" onClick={openPage}>
            {tr("openInPage")}
            <ArrowRight class="icon" aria-hidden="true" />
          </button>
        </footer>
      </aside>
    </div>
  );
}
