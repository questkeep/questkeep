import { createResource, For, Show } from "solid-js";
import type { ActivityItem } from "../bindings/ActivityItem";
import type { Game } from "../bindings/Game";
import { translator } from "../i18n";
import { activity as dict } from "../i18n/activity";
import { batchAction, changeCountText, entryAction, itemAt, sessionText } from "../lib/activity";
import * as api from "../lib/api";
import { formatAgo, formatDateTime } from "../lib/format";
import { store } from "../lib/store";
import { ActorChip } from "./ActivityItems";
import { Card } from "./GameSections";

const tr = translator(dict);

/** How many items the card shows. */
const RECENT = 3;

function line(item: ActivityItem): { text: string; count: string } {
  const names = store.activityNames;
  switch (item.type) {
    case "session":
      return { text: sessionText(item.durationSeconds), count: "" };
    case "batch":
      return { text: batchAction(item, names), count: "" };
    case "entry":
      return { text: entryAction(item, names), count: changeCountText(item.changeCount) };
  }
}

/** The game page's last few log lines and the way into the drawer with all of them. */
export function RecentActivityCard(props: { game: Game }) {
  // Reloads when this game (or the whole library) changes.
  let relevant = 0;
  const [items] = createResource(
    () => {
      const change = store.activityChange();
      if (change.gameIds.length === 0 || change.gameIds.includes(props.game.id)) {
        relevant = change.seq;
      }
      return [props.game.id, relevant] as const;
    },
    ([gameId]) =>
      Promise.resolve()
        .then(() =>
          api.activityPage({ gameId, actors: [], kinds: [], includeSessions: true, limit: RECENT }),
        )
        .then((page) => page?.items ?? [])
        .catch(() => [] as ActivityItem[]),
  );

  return (
    <Card
      title={tr("recentActivity")}
      order={9}
      class="details-card--recent-activity"
      action={
        <button
          type="button"
          class="link-btn alog-link"
          onClick={() => store.openActivityDrawer(props.game)}
        >
          {tr("showAll")}
        </button>
      }
    >
      <Show
        when={(items() ?? []).length > 0}
        fallback={
          <p class="muted alog-card__empty">{items.loading ? tr("loading") : tr("noActivity")}</p>
        }
      >
        <ul class="alog-card">
          <For each={items()}>
            {(item) => {
              const shown = line(item);
              return (
                <li class="alog-card__row">
                  <span class="alog-card__when" data-tooltip={formatDateTime(itemAt(item))}>
                    {formatAgo(itemAt(item))}
                  </span>
                  <span class="alog-card__text">
                    <ActorChip actor={item.type === "session" ? "session" : item.actor} />
                    <span>{shown.text}</span>
                    <Show when={shown.count}>
                      <small class="alog-row__detail">{shown.count}</small>
                    </Show>
                  </span>
                </li>
              );
            }}
          </For>
        </ul>
      </Show>
    </Card>
  );
}
