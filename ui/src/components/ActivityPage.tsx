import ChevronRight from "lucide-solid/icons/chevron-right";
import RefreshCw from "lucide-solid/icons/refresh-cw";
import { createEffect, createMemo, createSignal, For, on, Show } from "solid-js";
import type { ActivityCursor } from "../bindings/ActivityCursor";
import type { ActivityItem } from "../bindings/ActivityItem";
import type { SystemEvent } from "../bindings/SystemEvent";
import { translator } from "../i18n";
import { activity as dict } from "../i18n/activity";
import {
  DEFAULT_LIBRARY_FILTER,
  DEFAULT_SYSTEM_FILTER,
  isFiltered,
  isSystemFiltered,
  itemAt,
  itemKey,
  levelLabel,
  libraryQuery,
  systemActor,
  systemDetails,
  systemMessage,
  systemQuery,
  type LibraryFilter,
  type SystemFilter,
} from "../lib/activity";
import * as api from "../lib/api";
import { formatClockTime, formatDateTime } from "../lib/format";
import { store } from "../lib/store";
import { LibraryFilters, ScopePills, SystemFilters } from "./ActivityFilters";
import { ActorChip, ItemRow, PagedLog } from "./ActivityItems";

const tr = translator(dict);

type Tab = "library" | "system";

/**
 * The Activity page: what happened to the library (entries, batches, sessions) and, in the
 * System part, what Questkeep and its plugins did behind the scenes.
 */
export function ActivityPage() {
  const [tab, setTab] = createSignal<Tab>("library");
  const [library, setLibrary] = createSignal<LibraryFilter>(DEFAULT_LIBRARY_FILTER);
  const [system, setSystem] = createSignal<SystemFilter>(DEFAULT_SYSTEM_FILTER);
  const [labels, setLabels] = createSignal<{ game?: string; batch?: string }>({});
  // The refresh button and live updates.
  const [manual, setManual] = createSignal(0);

  // A way in (menu, toast, plugin row, drawer) asks for a filter.
  createEffect(
    on(store.activityRequest, (request) => {
      setTab(request.tab);
      setLibrary({ ...DEFAULT_LIBRARY_FILTER, ...request.library });
      setSystem({ ...DEFAULT_SYSTEM_FILTER, ...request.system });
      setLabels({ game: request.gameName, batch: request.batchLabel });
    }),
  );

  const loadLibrary = createMemo(() => {
    const filter = library();
    return (before: ActivityCursor | null) =>
      api.activityPage(libraryQuery(filter, new Date(), before));
  });
  const loadSystem = createMemo(() => {
    const filter = system();
    return (before: ActivityCursor | null) =>
      api.systemEvents(systemQuery(filter, new Date(), before));
  });
  const libraryRefresh = () => [store.activityChange().seq, manual()];
  // System events have no event of their own: they come with syncs, notifications and plugin
  // changes, so those (and the button) refresh the list.
  const systemRefresh = () => [
    store.activityChange().seq,
    store.notifications().length,
    store.notifications()[0]?.id,
    store.plugins(),
    store.syncState().active,
    manual(),
  ];

  const pills = () => {
    const list: { label: string; onRemove: () => void }[] = [];
    const filter = library();
    if (filter.gameId !== null) {
      list.push({
        label: tr("onlyGame", { name: labels().game ?? filter.gameId }),
        onRemove: () => setLibrary((f) => ({ ...f, gameId: null })),
      });
    }
    if (filter.batchId !== null) {
      list.push({
        label: tr("onlyBatch", { label: labels().batch ?? String(filter.batchId) }),
        onRemove: () => setLibrary((f) => ({ ...f, batchId: null })),
      });
    }
    return list;
  };

  return (
    <section class="settings activity-page" aria-label={tr("title")}>
      <header class="topbar">
        <h1 class="topbar__title">{tr("title")}</h1>
        <div class="segmented" role="radiogroup" aria-label={tr("part")}>
          <For each={["library", "system"] as const}>
            {(value) => (
              <button
                type="button"
                role="radio"
                class="segmented__item"
                classList={{ "segmented__item--on": tab() === value }}
                aria-checked={tab() === value}
                tabIndex={tab() === value ? 0 : -1}
                onClick={() => setTab(value)}
                onKeyDown={(event) => {
                  if (["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(event.key)) {
                    event.preventDefault();
                    const other = value === "library" ? "system" : "library";
                    setTab(other);
                    (
                      event.currentTarget.parentElement?.querySelector(
                        `[data-tab="${other}"]`,
                      ) as HTMLElement | null
                    )?.focus();
                  }
                }}
                data-tab={value}
              >
                {value === "library" ? tr("tabLibrary") : tr("tabSystem")}
              </button>
            )}
          </For>
        </div>
        <span class="topbar__spacer" />
        <button
          type="button"
          class="icon-btn"
          aria-label={tr("refresh")}
          data-tooltip={tr("refresh")}
          onClick={() => setManual((n) => n + 1)}
        >
          <RefreshCw class="icon" aria-hidden="true" />
        </button>
      </header>
      <div class="activity-page__scroll">
        <div class="activity-page__content">
          <Show
            when={tab() === "library"}
            fallback={
              <>
                <p class="activity-page__sub">{tr("subtitleSystem")}</p>
                <SystemFilters
                  filter={system()}
                  onChange={(patch) => setSystem((f) => ({ ...f, ...patch }))}
                />
                <PagedLog<SystemEvent>
                  load={loadSystem()}
                  refresh={systemRefresh}
                  key={(event) => String(event.id)}
                  at={(event) => event.at}
                  label={tr("tabSystem")}
                  empty={isSystemFiltered(system()) ? tr("emptyFiltered") : tr("empty")}
                  row={(event) => <SystemRow event={event} />}
                />
              </>
            }
          >
            <p class="activity-page__sub">{tr("subtitleLibrary")}</p>
            <LibraryFilters
              filter={library()}
              onChange={(patch) => setLibrary((f) => ({ ...f, ...patch }))}
            />
            <ScopePills pills={pills()} />
            <PagedLog<ActivityItem>
              load={loadLibrary()}
              refresh={libraryRefresh}
              key={itemKey}
              at={itemAt}
              label={tr("tabLibrary")}
              empty={isFiltered(library()) ? tr("emptyFiltered") : tr("empty")}
              row={(item) => <ItemRow item={item} showGame />}
            />
          </Show>
        </div>
      </div>
    </section>
  );
}

/** One system event: level, time, source, message; expands to its details. */
function SystemRow(props: { event: SystemEvent }) {
  const [open, setOpen] = createSignal(false);
  const message = () => systemMessage(props.event, store.activityNames);
  const details = () => systemDetails(props.event);
  return (
    <li class="alog-row alog-row--system" classList={{ "alog-row--open": open() }}>
      <div
        class="alog-row__head"
        onClick={(event) => {
          if ((event.target as Element).closest("button") === null) setOpen(!open());
        }}
      >
        <button
          type="button"
          class="alog-row__toggle"
          data-activity-row
          aria-expanded={open()}
          aria-label={`${levelLabel(props.event.level)} · ${message()}`}
          onClick={() => setOpen(!open())}
        >
          <ChevronRight class="icon alog-row__chev" aria-hidden="true" />
          <span class={`alog-level alog-level--${props.event.level}`}>
            {levelLabel(props.event.level)}
          </span>
          <span class="alog-row__time">{formatClockTime(new Date(props.event.at).getTime())}</span>
          <ActorChip actor={systemActor(props.event)} />
        </button>
        <span class="alog-row__summary">
          <span class="alog-row__action">{message()}</span>
        </span>
      </div>
      <Show when={open()}>
        <div class="alog-row__details">
          <p class="alog-note">{formatDateTime(props.event.at)}</p>
          <Show when={message() !== props.event.message}>
            <p class="alog-mono selectable">{props.event.message}</p>
          </Show>
          <Show when={details().length > 0} fallback={<p class="alog-note">{tr("noDetails")}</p>}>
            <dl class="alog-kv selectable">
              <For each={details()}>
                {(line) => (
                  <>
                    <dt>{line.key}</dt>
                    <dd>{line.value}</dd>
                  </>
                )}
              </For>
            </dl>
          </Show>
        </div>
      </Show>
    </li>
  );
}
