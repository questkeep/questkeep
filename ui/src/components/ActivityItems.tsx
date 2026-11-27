import ChevronRight from "lucide-solid/icons/chevron-right";
import {
  createEffect,
  createMemo,
  createSignal,
  For,
  Match,
  on,
  onCleanup,
  onMount,
  Show,
  Switch,
  type JSX,
} from "solid-js";
import type { ActivityChange } from "../bindings/ActivityChange";
import type { ActivityCursor } from "../bindings/ActivityCursor";
import type { ActivityDetails } from "../bindings/ActivityDetails";
import type { ActivityItem } from "../bindings/ActivityItem";
import type { Actor } from "../bindings/Actor";
import { translator } from "../i18n";
import { activity as dict } from "../i18n/activity";
import {
  actorLabel,
  appendPage,
  batchAction,
  batchCounts,
  batchDetail,
  changeCountText,
  changeView,
  entryAction,
  entryDetail,
  fullValue,
  groupByDay,
  itemAt,
  itemKey,
  sentence,
  sessionText,
  type BatchItem,
  type EntryItem,
  type SessionItem,
} from "../lib/activity";
import * as api from "../lib/api";
import { errorMessage } from "../lib/errors";
import { formatClockTime } from "../lib/format";
import { store } from "../lib/store";

const tr = translator(dict);

const clock = (iso: string) => formatClockTime(new Date(iso).getTime());

/** Who made a change, as a small colored label. */
export function ActorChip(props: { actor: Actor | "session" }) {
  const kind = () => (props.actor === "session" ? "session" : props.actor.kind);
  return (
    <span class={`alog-actor alog-actor--${kind()}`}>
      {props.actor === "session" ? tr("session") : actorLabel(props.actor, store.activityNames)}
    </span>
  );
}

/** Opens a game's page; a game that is gone says so instead. */
export async function openActivityGame(id: string, name: string) {
  try {
    const game = await api.getGame(id);
    store.openGamePage(game);
  } catch {
    store.pushToast("info", tr("gameGone", { name }));
  }
}

/** A game's name that opens its page (plain text for library-level entries and removed games). */
export function GameName(props: { id: string | null; name: string; plain?: boolean }) {
  return (
    <Show when={props.id !== null && !props.plain} fallback={<b class="alog-game">{props.name}</b>}>
      <button
        type="button"
        class="alog-game alog-game--link"
        data-tooltip={tr("openGame", { name: props.name })}
        onClick={(event) => {
          event.stopPropagation();
          void openActivityGame(props.id ?? "", props.name);
        }}
      >
        {props.name}
      </button>
    </Show>
  );
}

/** The field changes of one entry: before → after, + added / − removed, or a word. */
export function ChangeTable(props: { changes: ActivityChange[] }) {
  return (
    <Show when={props.changes.length > 0} fallback={<p class="alog-note">{tr("noFields")}</p>}>
      <table class="alog-diff">
        <tbody>
          <For each={props.changes}>
            {(change) => {
              const view = changeView(change, store.activityNames);
              return (
                <tr>
                  <th scope="row">{view.label}</th>
                  <td>
                    <Switch>
                      <Match when={view.mode === "value" && view}>
                        {(v) => (
                          <>
                            <Show when={v().before !== null}>
                              <del
                                class="alog-old"
                                data-tooltip={fullValue(change.before) ?? undefined}
                              >
                                {v().before}
                              </del>
                              <span class="alog-arrow" aria-label={tr("after")}>
                                →
                              </span>
                            </Show>
                            <ins
                              class="alog-new"
                              classList={{ "alog-none": v().after === null }}
                              data-tooltip={fullValue(change.after) ?? undefined}
                            >
                              {v().after ?? tr("none")}
                            </ins>
                          </>
                        )}
                      </Match>
                      <Match when={view.mode === "set" && view}>
                        {(v) => (
                          <span class="alog-set">
                            <For each={v().added}>
                              {(item) => (
                                <ins class="alog-new" aria-label={`${tr("addedItems")}: ${item}`}>
                                  + {item}
                                </ins>
                              )}
                            </For>
                            <For each={v().removed}>
                              {(item) => (
                                <del class="alog-old" aria-label={`${tr("removedItems")}: ${item}`}>
                                  − {item}
                                </del>
                              )}
                            </For>
                          </span>
                        )}
                      </Match>
                      <Match when={view.mode === "changed" && view}>
                        {(v) => <span>{v().text}</span>}
                      </Match>
                    </Switch>
                  </td>
                </tr>
              );
            }}
          </For>
        </tbody>
      </table>
    </Show>
  );
}

/**
 * One line of the log with its toggle. The whole head expands the row on click; the first part
 * is the focusable toggle button (game links stay separate buttons).
 */
function Row(props: {
  class?: string;
  /** What the row says, for screen readers (actor, game, action, count). */
  label: string;
  time: string | null;
  chip: JSX.Element;
  game?: JSX.Element;
  summary: JSX.Element;
  count?: string;
  /** Without it, the row doesn't expand (sessions). */
  details?: () => JSX.Element;
  onOpen?: () => void;
}) {
  const [open, setOpen] = createSignal(false);
  const expandable = () => props.details !== undefined;
  function toggle() {
    if (!expandable()) return;
    const next = !open();
    setOpen(next);
    if (next) props.onOpen?.();
  }
  return (
    <li class={`alog-row ${props.class ?? ""}`} classList={{ "alog-row--open": open() }}>
      <div
        class="alog-row__head"
        classList={{ "alog-row__head--static": !expandable() }}
        onClick={(event) => {
          // A click on the toggle button itself already toggled.
          if ((event.target as Element).closest("button") === null) toggle();
        }}
      >
        <button
          type="button"
          class="alog-row__toggle"
          data-activity-row
          aria-expanded={expandable() ? open() : undefined}
          aria-label={props.label}
          tabIndex={0}
          onClick={toggle}
        >
          <ChevronRight
            class="icon alog-row__chev"
            classList={{ "alog-row__chev--hidden": !expandable() }}
            aria-hidden="true"
          />
          <Show when={props.time}>
            <span class="alog-row__time">{props.time}</span>
          </Show>
          {props.chip}
        </button>
        <span class="alog-row__summary">
          <Show when={props.game}>
            {props.game}
            <span class="alog-row__sep">·</span>
          </Show>
          {props.summary}
        </span>
        <Show when={props.count}>
          <span class="alog-row__count" aria-hidden="true">
            {props.count}
          </span>
        </Show>
      </div>
      <Show when={open() && props.details}>
        {(details) => <div class="alog-row__details">{details()()}</div>}
      </Show>
    </li>
  );
}

function Summary(props: { action: string; detail: string | null }) {
  return (
    <>
      <span class="alog-row__action">{props.action}</span>
      <Show when={props.detail}>
        <small class="alog-row__detail">{props.detail}</small>
      </Show>
    </>
  );
}

/** One change of one game (or of the library): expands to its field changes. */
export function EntryRow(props: {
  entry: EntryItem;
  showGame: boolean;
  /** Inside a batch: the batch already says who and when. */
  nested?: boolean;
}) {
  const [details, setDetails] = createSignal<ActivityDetails | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  const names = store.activityNames;
  const action = () => entryAction(props.entry, names);
  const detail = () => (props.nested ? null : entryDetail(props.entry, names));
  const actor = () => actorLabel(props.entry.actor, names);
  const hasGame = () => props.showGame && props.entry.gameName !== "";
  const label = () =>
    [
      sentence(actor(), action()),
      hasGame() ? props.entry.gameName : null,
      detail(),
      changeCountText(props.entry.changeCount),
    ]
      .filter((part) => part)
      .join(" · ");

  function load() {
    if (details() !== null) return;
    api
      .activityDetails(props.entry.id)
      .then(setDetails)
      .catch((e: unknown) => setError(errorMessage(e)));
  }

  return (
    <Row
      class={props.nested ? "alog-row--nested" : undefined}
      label={label()}
      time={props.nested ? null : clock(props.entry.at)}
      chip={props.nested ? <></> : <ActorChip actor={props.entry.actor} />}
      game={
        hasGame() ? (
          <GameName
            id={props.entry.gameId}
            name={props.entry.gameName}
            plain={props.entry.kind === "removed"}
          />
        ) : undefined
      }
      summary={<Summary action={action()} detail={detail()} />}
      count={changeCountText(props.entry.changeCount)}
      onOpen={load}
      details={() => (
        <>
          <Show when={error()}>{(message) => <p class="alog-error">{message()}</p>}</Show>
          <Show when={!props.nested && details()?.batch}>
            {(batch) => (
              <p class="alog-note">
                {tr("partOf", {
                  label: `${actorLabel(batch().actor, names)} · ${batchAction(batch(), names)} · ${clock(batch().startedAt)}`,
                })}
              </p>
            )}
          </Show>
          <Show
            when={details()}
            fallback={
              <Show when={!error()}>
                <p class="alog-note">{tr("loading")}</p>
              </Show>
            }
          >
            {(loaded) => <ChangeTable changes={loaded().changes} />}
          </Show>
        </>
      )}
    />
  );
}

/** Changes made together: expands to the games it touched, each expanding to its changes. */
export function BatchRow(props: { batch: BatchItem }) {
  const names = store.activityNames;
  const action = () => batchAction(props.batch, names);
  const detail = () => batchDetail(props.batch, names);
  const count = () => batchCounts(props.batch);
  return (
    <Row
      class="alog-row--batch"
      label={[sentence(actorLabel(props.batch.actor, names), action()), detail(), count()]
        .filter((part) => part)
        .join(" · ")}
      time={clock(props.batch.startedAt)}
      chip={<ActorChip actor={props.batch.actor} />}
      summary={<Summary action={action()} detail={detail()} />}
      count={count()}
      details={() => <BatchEntries batchId={props.batch.id} />}
    />
  );
}

function BatchEntries(props: { batchId: number }) {
  const [items, setItems] = createSignal<EntryItem[]>([]);
  const [next, setNext] = createSignal<ActivityCursor | null>(null);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  async function load(before: ActivityCursor | null) {
    setLoading(true);
    try {
      const page = await api.activityBatchEntries(props.batchId, before);
      const entries = page.items.filter((item): item is EntryItem => item.type === "entry");
      setItems((shown) => appendPage(shown, entries, itemKey));
      setNext(page.next);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setLoading(false);
    }
  }
  onMount(() => void load(null));

  return (
    <>
      <p class="alog-note">{tr("batchNote")}</p>
      <Show when={error()}>{(message) => <p class="alog-error">{message()}</p>}</Show>
      <ul class="alog-list alog-list--nested">
        <For each={items()}>{(entry) => <EntryRow entry={entry} showGame nested />}</For>
      </ul>
      <Show when={loading()}>
        <p class="alog-note">{tr("loading")}</p>
      </Show>
      <Show when={!loading() && next()}>
        {(cursor) => (
          <button
            type="button"
            class="btn btn--small alog-more"
            onClick={() => void load(cursor())}
          >
            {tr("loadMore")}
          </button>
        )}
      </Show>
    </>
  );
}

/** A play session: a quieter line, read from the session list. */
export function SessionRow(props: { session: SessionItem; showGame: boolean }) {
  const storeName = () => store.activityNames.store(props.session.store);
  return (
    <Row
      class="alog-row--session"
      label={[
        tr("session"),
        props.showGame ? props.session.gameName : null,
        sessionText(props.session.durationSeconds),
        storeName(),
      ]
        .filter((part) => part)
        .join(" · ")}
      time={clock(props.session.startedAt)}
      chip={<ActorChip actor="session" />}
      game={
        props.showGame ? (
          <GameName id={props.session.gameId} name={props.session.gameName} />
        ) : undefined
      }
      summary={<span class="alog-row__action">{sessionText(props.session.durationSeconds)}</span>}
      count={storeName()}
    />
  );
}

/** Any item of the timeline. */
export function ItemRow(props: { item: ActivityItem; showGame: boolean }) {
  return (
    <Switch>
      <Match when={props.item.type === "entry" && (props.item as EntryItem)}>
        {(entry) => <EntryRow entry={entry()} showGame={props.showGame} />}
      </Match>
      <Match when={props.item.type === "batch" && (props.item as BatchItem)}>
        {(batch) => <BatchRow batch={batch()} />}
      </Match>
      <Match when={props.item.type === "session" && (props.item as SessionItem)}>
        {(session) => <SessionRow session={session()} showGame={props.showGame} />}
      </Match>
    </Switch>
  );
}

/** Up/Down (and Home/End) move between the rows of a log. */
export function onRowKeys(event: KeyboardEvent) {
  const target = event.target as HTMLElement;
  if (!target.matches("[data-activity-row]")) return;
  const rows = Array.from(
    (event.currentTarget as HTMLElement).querySelectorAll<HTMLElement>("[data-activity-row]"),
  );
  const index = rows.indexOf(target);
  let next: number | null = null;
  if (event.key === "ArrowDown") next = Math.min(rows.length - 1, index + 1);
  else if (event.key === "ArrowUp") next = Math.max(0, index - 1);
  else if (event.key === "Home") next = 0;
  else if (event.key === "End") next = rows.length - 1;
  if (next === null) return;
  event.preventDefault();
  rows[next]?.focus();
}

/** Merges a fresh first page into what is shown: new items on top, changed ones replaced. */
export function mergeFresh<T>(
  shown: readonly T[],
  fresh: readonly T[],
  key: (item: T) => string,
): T[] {
  const byKey = new Map(fresh.map((item) => [key(item), item]));
  const kept = shown.map((item) => {
    const update = byKey.get(key(item));
    return update && JSON.stringify(update) !== JSON.stringify(item) ? update : item;
  });
  const known = new Set(shown.map(key));
  return [...fresh.filter((item) => !known.has(key(item))), ...kept];
}

/**
 * A paged, day-grouped list: loads the first page when `query` changes, the next one when the
 * end comes into view (or "Load more" is chosen), and merges a fresh first page on `refresh`.
 */
export function PagedLog<T extends { cursor: ActivityCursor }>(props: {
  /** Loads a page; its identity changes when the filter does. */
  load: (before: ActivityCursor | null) => Promise<{ items: T[]; next: ActivityCursor | null }>;
  /** Counts the reasons to merge a fresh first page (live updates). */
  refresh: () => unknown;
  key: (item: T) => string;
  at: (item: T) => string;
  row: (item: T) => JSX.Element;
  empty: string;
  label: string;
}) {
  const [items, setItems] = createSignal<T[]>([]);
  const [next, setNext] = createSignal<ActivityCursor | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [loaded, setLoaded] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  let seq = 0;
  const [sentinel, setSentinel] = createSignal<HTMLButtonElement>();

  async function first() {
    const mine = ++seq;
    setLoading(true);
    setError(null);
    try {
      const page = await props.load(null);
      if (mine !== seq) return;
      setItems(page.items);
      setNext(page.next);
    } catch (e) {
      if (mine === seq) setError(errorMessage(e));
    } finally {
      if (mine === seq) {
        setLoading(false);
        setLoaded(true);
      }
    }
  }

  async function more() {
    const cursor = next();
    if (!cursor || loading()) return;
    const mine = seq;
    setLoading(true);
    try {
      const page = await props.load(cursor);
      if (mine !== seq) return;
      setItems((shown) => appendPage(shown, page.items, props.key));
      setNext(page.next);
    } catch (e) {
      if (mine === seq) setError(errorMessage(e));
    } finally {
      if (mine === seq) setLoading(false);
    }
  }

  async function fresh() {
    const mine = seq;
    try {
      const page = await props.load(null);
      if (mine !== seq) return;
      setItems((shown) => mergeFresh(shown, page.items, props.key));
      if (items().length === page.items.length) setNext(page.next);
    } catch {
      // A live update that fails leaves the list as it was.
    }
  }

  createEffect(
    on(
      () => props.load,
      () => void first(),
    ),
  );
  createEffect(on(props.refresh, () => void fresh(), { defer: true }));

  onMount(() => {
    if (typeof IntersectionObserver === "undefined") return;
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((e) => e.isIntersecting)) void more();
    });
    createEffect(() => {
      observer.disconnect();
      const target = sentinel();
      if (next() && target?.isConnected) observer.observe(target);
    });
    onCleanup(() => observer.disconnect());
  });

  const groups = createMemo(() => groupByDay(items(), props.at, new Date()));

  return (
    <div class="alog" onKeyDown={onRowKeys}>
      <Show when={error()}>
        {(message) => <p class="alog-error">{tr("loadFailed", { error: message() })}</p>}
      </Show>
      <Show when={loaded() && items().length === 0 && !error()}>
        <p class="alog-empty">{props.empty}</p>
      </Show>
      <For each={groups()}>
        {(group) => (
          <section class="alog-day" aria-label={group.label}>
            <h3 class="alog-day__title">{group.label}</h3>
            <ul class="alog-list" aria-label={`${props.label} · ${group.label}`}>
              <For each={group.items}>{(item) => props.row(item)}</For>
            </ul>
          </section>
        )}
      </For>
      <Show when={loading()}>
        <p class="alog-note" role="status">
          {tr("loading")}
        </p>
      </Show>
      <Show when={next() && !loading()}>
        <button
          ref={setSentinel}
          type="button"
          class="btn btn--small alog-more"
          onClick={() => void more()}
        >
          {tr("loadMore")}
        </button>
      </Show>
    </div>
  );
}

export { itemAt, itemKey };
