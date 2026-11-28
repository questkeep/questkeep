import Dices from "lucide-solid/icons/dices";
import Download from "lucide-solid/icons/download";
import Play from "lucide-solid/icons/play";
import { createMemo, createSignal, Show } from "solid-js";
import type { Game } from "../bindings/Game";
import { translator } from "../i18n";
import { randomGame } from "../i18n/randomGame";
import { shellCommandBar } from "../i18n/shellCommandBar";
import { installWithToast, launchWithToast } from "../lib/actions";
import { formatAgo, formatPlaytime } from "../lib/format";
import { primaryAction } from "../lib/primaryAction";
import { nextPick, randomPool } from "../lib/randomPick";
import { store } from "../lib/store";
import { shownName } from "../lib/views";
import { Cover } from "./Cover";
import { Modal } from "./Modal";
import { sourceName, SourceIcon } from "./SourceIcon";

const tr = translator(randomGame);
const trBar = translator(shellCommandBar);

// Kept while the app runs: someone who unticked it once wants all games next time too.
const [installedOnly, setInstalledOnly] = createSignal(true);

/**
 * Picks a random game from what the open view shows (its filters and the filter box apply):
 * Play (or Install), Open page, or Pick another, which shows every game once before repeating.
 */
export function RandomGameDialog(props: { onClose: () => void }) {
  // The view's games when the dialog opened; a sync meanwhile does not change the pool.
  const games: readonly Game[] = store.games();
  const view = store.activeView();
  const pool = createMemo(() => randomPool(games, installedOnly()));
  const [seen, setSeen] = createSignal<string[]>([]);
  const [game, setGame] = createSignal<Game | null>(null);

  function pickAnother() {
    const next = nextPick(pool(), seen());
    setSeen(next.seen);
    setGame(next.game);
  }
  pickAnother();

  const action = () => {
    const current = game();
    return current ? primaryAction(current, store.isRunning(current.id)) : null;
  };

  function act() {
    const current = game();
    if (!current) return;
    props.onClose();
    if (action() === "install") void installWithToast(current.id);
    else if (action() === "play") void launchWithToast(current.id);
    else store.openGamePage(current);
  }

  function openPage() {
    const current = game();
    if (!current) return;
    props.onClose();
    store.openGamePage(current);
  }

  return (
    <Modal
      title={tr("title")}
      onClose={props.onClose}
      footer={
        <div class="form__actions">
          <button
            type="button"
            class="btn"
            data-autofocus
            disabled={pool().length === 0}
            onClick={pickAnother}
          >
            <Dices class="icon" aria-hidden="true" />
            {tr("pickAnother")}
          </button>
          <button type="button" class="btn" disabled={!game()} onClick={openPage}>
            {tr("openPage")}
          </button>
          <Show when={action() === "play" || action() === "install"}>
            <button type="button" class="btn btn--primary" onClick={act}>
              <Show
                when={action() === "play"}
                fallback={<Download class="icon" aria-hidden="true" />}
              >
                <Play class="icon" aria-hidden="true" />
              </Show>
              {action() === "play"
                ? trBar(game()?.kind === "app" ? "start" : "play")
                : trBar("install")}
            </button>
          </Show>
        </div>
      }
    >
      <p class="muted random__from">
        {tr("from", { view: view ? shownName(view) : "", count: pool().length })}
      </p>
      <Show
        when={game()}
        fallback={<p class="random__empty">{tr(games.length > 0 ? "noneInstalled" : "none")}</p>}
      >
        {(current) => (
          <div class="random__pick" aria-live="polite">
            <div class="random__cover">
              <Cover game={current()} />
            </div>
            <div class="random__info">
              <h3 class="random__name">{current().name}</h3>
              <p class="random__meta">
                <SourceIcon source={current().source} />
                {sourceName(current().source)} ·{" "}
                {current().installed ? tr("installed") : tr("notInstalled")}
              </p>
              <Show when={current().playtimeSeconds > 0}>
                <p class="muted">
                  {tr("played", { time: formatPlaytime(current().playtimeSeconds) })}
                </p>
              </Show>
              <p class="muted">
                {current().lastPlayed
                  ? tr("lastStarted", { when: formatAgo(current().lastPlayed) })
                  : tr("neverStarted")}
              </p>
            </div>
          </div>
        )}
      </Show>
      <label class="check random__installed">
        <input
          type="checkbox"
          checked={installedOnly()}
          onChange={(event) => {
            setInstalledOnly(event.currentTarget.checked);
            setSeen([]);
            pickAnother();
          }}
        />
        {tr("onlyInstalled")}
      </label>
    </Modal>
  );
}
