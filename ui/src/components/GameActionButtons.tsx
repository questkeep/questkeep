import Ellipsis from "lucide-solid/icons/ellipsis";
import {
  createEffect,
  createResource,
  createSignal,
  For,
  onCleanup,
  onMount,
  Show,
  type JSX,
} from "solid-js";
import type { Game } from "../bindings/Game";
import type { GameTool } from "../bindings/GameTool";
import { gameTools, runGameTool } from "../lib/api";
import type { ActionWidths } from "../lib/actionRow";
import { errorMessage } from "../lib/errors";
import { store } from "../lib/store";
import { translator } from "../i18n";
import { scriptsText } from "../i18n/scripts";
import { createGameActions, runAction } from "../lib/gameActions";
import { createToolIcons } from "../lib/toolIcons";
import { Menu } from "./Menu";
import type { PopoverAnchor } from "./Popover";
import { ScriptIcon } from "./ScriptIcon";
import { ToolIcon } from "./EditToolsList";

const tr = translator(scriptsText);

/** A button of the row: a tool or an action script. */
interface ActionItem {
  key: string;
  name: string;
  icon: () => JSX.Element;
  run: () => void;
}

async function runTool(tool: GameTool, gameId: string): Promise<void> {
  try {
    await runGameTool(gameId, tool.id);
  } catch (failure) {
    store.pushToast("error", errorMessage(failure));
  }
}

const exactWidth = (element: Element) => element.getBoundingClientRect().width;

/** The widths of the measuring copy's buttons of one kind (`label`, `icon` or `more`). */
const measured = (box: Element | undefined, kind: string) =>
  [...(box?.querySelectorAll(`[data-measure="${kind}"]`) ?? [])].map(exactWidth);

/**
 * The tools and action scripts of a game on its page (tools first), next to Play. The page says
 * how they fit: with their names (`labels`) or as icons with the name as tooltip, and how many
 * are shown; the rest are under "…". Without the page's say, the first two show with names.
 * `onMeasure` gets the buttons' widths both ways (from a hidden copy), so the page can decide.
 * Nothing is drawn for a game without any.
 */
export function GameActionButtons(props: {
  game: Game;
  labels?: boolean;
  shown?: number;
  onMeasure?: (widths: ActionWidths) => void;
}) {
  const actions = createGameActions(() => props.game.id);
  // Loaded again whenever the game changes (saving tools reports a change of the game).
  const [tools] = createResource(
    () => props.game,
    (game) =>
      Promise.resolve()
        .then(() => gameTools(game.id))
        .then((list) => ({ id: game.id, list: list ?? [] }))
        .catch(() => ({ id: game.id, list: [] as GameTool[] })),
  );
  const toolIcons = createToolIcons(() => props.game);
  const items = (): ActionItem[] => {
    const loaded = tools();
    const ownTools = loaded && loaded.id === props.game.id ? loaded.list : [];
    return [
      ...ownTools.map((tool) => ({
        key: `tool:${tool.id}`,
        name: tool.name,
        icon: () => <ToolIcon kind={tool.action.kind} src={toolIcons().get(tool.id)} />,
        run: () => void runTool(tool, props.game.id),
      })),
      ...actions().map((script) => ({
        key: `script:${script.id}`,
        name: script.name,
        icon: () => <ScriptIcon name={script.icon} />,
        run: () => void runAction(script, props.game.id),
      })),
    ];
  };
  const labels = () => props.labels ?? true;
  const shownCount = () => Math.min(items().length, props.shown ?? 2);
  const shown = () => items().slice(0, shownCount());
  const more = () => items().slice(shownCount());
  const [menu, setMenu] = createSignal<PopoverAnchor | null>(null);
  let moreButton: HTMLButtonElement | undefined;
  let measureBox: HTMLDivElement | undefined;

  function measure() {
    if (!props.onMeasure) return;
    props.onMeasure({
      labels: measured(measureBox, "label"),
      icons: measured(measureBox, "icon"),
      more: measured(measureBox, "more")[0] ?? 0,
    });
  }
  // A game without actions reports none; with actions the hidden copy is measured.
  createEffect(() => {
    if (items().length === 0) props.onMeasure?.({ labels: [], icons: [], more: 0 });
  });
  const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(measure);
  onCleanup(() => observer?.disconnect());

  return (
    <Show when={items().length > 0}>
      <div class="script-actions" role="group" aria-label={tr("actions")}>
        <For each={shown()}>
          {(item) => (
            <Show
              when={labels()}
              fallback={
                <button
                  type="button"
                  class="toggle-btn"
                  aria-label={item.name}
                  data-tooltip={item.name}
                  onClick={() => item.run()}
                >
                  {item.icon()}
                </button>
              }
            >
              <button type="button" class="btn" onClick={() => item.run()}>
                {item.icon()}
                {item.name}
              </button>
            </Show>
          )}
        </For>
        <Show when={more().length > 0}>
          <button
            ref={(el) => (moreButton = el)}
            type="button"
            class="toggle-btn"
            aria-label={tr("moreActions")}
            data-tooltip={tr("moreActions")}
            aria-haspopup="menu"
            aria-expanded={menu() !== null}
            onClick={() =>
              setMenu(
                menu() === null && moreButton
                  ? { rect: moreButton.getBoundingClientRect(), align: "start" }
                  : null,
              )
            }
          >
            <Ellipsis class="icon" aria-hidden="true" />
          </button>
        </Show>
        {/* A hidden copy with every button both ways, for the page to measure. */}
        <Show when={props.onMeasure}>
          <div
            ref={(el) => {
              measureBox = el;
              onMount(() => {
                measure();
                observer?.observe(el);
              });
            }}
            class="script-actions__measure"
            aria-hidden="true"
            inert
          >
            <For each={items()}>
              {(item) => (
                <>
                  <span class="btn" data-measure="label">
                    {item.icon()}
                    {item.name}
                  </span>
                  <span class="toggle-btn" data-measure="icon">
                    {item.icon()}
                  </span>
                </>
              )}
            </For>
            <span class="toggle-btn" data-measure="more">
              <Ellipsis class="icon" />
            </span>
          </div>
        </Show>
      </div>
      <Show when={menu()}>
        {(anchor) => (
          <Menu
            anchor={anchor()}
            label={tr("moreActions")}
            inside={() => [moreButton]}
            onClose={(returnFocus) => {
              setMenu(null);
              if (returnFocus) moreButton?.focus();
            }}
            entries={more().map((item) => ({
              label: item.name,
              icon: item.icon,
              onSelect: () => item.run(),
            }))}
          />
        )}
      </Show>
    </Show>
  );
}
