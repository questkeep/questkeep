import { translator } from "../i18n";
import { gameLaunch } from "../i18n/gameLaunch";
import Ellipsis from "lucide-solid/icons/ellipsis";
import Globe from "lucide-solid/icons/globe";
import GripVertical from "lucide-solid/icons/grip-vertical";
import Plus from "lucide-solid/icons/plus";
import AppWindow from "lucide-solid/icons/app-window";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  createEffect,
  createResource,
  createSignal,
  createUniqueId,
  For,
  on,
  onCleanup,
  Show,
} from "solid-js";
import { Portal } from "solid-js/web";
import type { ActionKind } from "../bindings/ActionKind";
import type { Game } from "../bindings/Game";
import type { GameTool } from "../bindings/GameTool";
import { importToolIcon, toolDefaultIcon } from "../lib/api";
import { moveItem } from "../lib/editGame";
import { errorMessage } from "../lib/errors";
import { pickDirectory, pickExecutable, pickToolIcon } from "../lib/pickers";
import { store } from "../lib/store";
import { createToolIcons } from "../lib/toolIcons";
import { ImagePicker } from "./ImagePicker";
import { Menu } from "./Menu";
import type { PopoverAnchor } from "./Popover";

const tr = translator(gameLaunch);

function emptyToNull(value: string): string | null {
  const trimmed = value.trim();
  return trimmed === "" ? null : trimmed;
}

/** A tool's icon: its picture (`src`), else a program or a link symbol. */
export function ToolIcon(props: { kind: ActionKind; src?: string | null }) {
  const [failed, setFailed] = createSignal(false);
  createEffect(
    on(
      () => props.src,
      () => setFailed(false),
      { defer: true },
    ),
  );
  const symbol = () => (
    <Show when={props.kind === "uri"} fallback={<AppWindow class="icon" aria-hidden="true" />}>
      <Globe class="icon" aria-hidden="true" />
    </Show>
  );
  return (
    <Show when={!failed() && props.src} fallback={symbol()}>
      {(src) => (
        <img
          class="tool-icon"
          src={src()}
          alt=""
          draggable={false}
          onError={() => setFailed(true)}
        />
      )}
    </Show>
  );
}

/** What a tool's row shows under its name. */
const detail = (tool: GameTool) =>
  tool.action.args ? `${tool.action.target} ${tool.action.args}` : tool.action.target;

/** Typing in the program or link field waits this long before its icon is looked up. */
const ICON_LOOKUP_DELAY_MS = 400;

function ToolForm(props: {
  gameId: string;
  tool: GameTool | null;
  /** The tool's current picture, if it has one. */
  iconSrc: string | undefined;
  /** Saves the tool; `src` is the picture the form showed for it. */
  onSave: (tool: GameTool, src: string | undefined) => void;
  onCancel: () => void;
}) {
  // A new tool gets its id now: a picked icon is stored under it.
  const id = props.tool?.id ?? crypto.randomUUID();
  const [icon, setIcon] = createSignal<string | null>(props.tool?.icon ?? null);
  /** A picture picked in this form (shown at once). */
  const [pickedSrc, setPickedSrc] = createSignal<string | undefined>();
  const [name, setName] = createSignal(props.tool?.name ?? "");
  const [kind, setKind] = createSignal<ActionKind>(props.tool?.action.kind ?? "file");
  const [target, setTarget] = createSignal(props.tool?.action.target ?? "");
  // The program or link the automatic icon is looked up for, once typing pauses.
  const [lookup, setLookup] = createSignal({ kind: kind(), target: target().trim() });
  createEffect(
    on(
      [kind, target],
      ([k, t]) => {
        const timer = setTimeout(
          () => setLookup({ kind: k, target: t.trim() }),
          ICON_LOOKUP_DELAY_MS,
        );
        onCleanup(() => clearTimeout(timer));
      },
      { defer: true },
    ),
  );
  const [automatic] = createResource(
    () => ({ ...lookup(), version: store.siteIconsVersion() }),
    (asked) =>
      asked.target === ""
        ? Promise.resolve(undefined)
        : Promise.resolve()
            .then(() =>
              toolDefaultIcon(props.gameId, {
                id,
                name: "",
                icon: null,
                action: {
                  kind: asked.kind,
                  target: asked.target,
                  args: null,
                  workingDir: null,
                  tracking: "default",
                  trackingPath: null,
                },
              }),
            )
            .then((path) => (path ? `${convertFileSrc(path)}?v=${asked.version}` : undefined))
            .catch(() => undefined),
  );
  // The picked picture, the tool's own, or its automatic one (program icon, favicon); the saved
  // picture stands in until the lookup answers.
  const previewSrc = () => {
    const current = icon();
    if (current !== null) {
      return pickedSrc() ?? (current === props.tool?.icon ? props.iconSrc : undefined);
    }
    if (automatic.state === "ready") return automatic();
    return props.tool?.icon === null ? props.iconSrc : undefined;
  };
  const [args, setArgs] = createSignal(props.tool?.action.args ?? "");
  const [workingDir, setWorkingDir] = createSignal(props.tool?.action.workingDir ?? "");
  const [error, setError] = createSignal<string | null>(null);
  const typeId = createUniqueId();
  const targetId = createUniqueId();
  const iconId = createUniqueId();

  const [searching, setSearching] = createSignal(false);

  async function chooseIcon() {
    const path = await pickToolIcon();
    if (path !== null) await useIcon(path);
  }

  /** Keeps a picked file or web picture as the tool's icon (copied now, kept on Save). */
  async function useIcon(source: string) {
    try {
      const picked = await importToolIcon(id, source);
      const src = convertFileSrc(picked.path);
      setIcon(picked.icon);
      setPickedSrc(src);
      setError(null);
    } catch (failure) {
      setError(errorMessage(failure));
    }
  }

  function save(event: SubmitEvent) {
    event.preventDefault();
    event.stopPropagation();
    const file = kind() === "file";
    if (target().trim() === "") {
      setError(file ? tr("chooseProgram") : tr("chooseLink"));
      return;
    }
    props.onSave(
      {
        id,
        name: name().trim(),
        icon: icon(),
        action: {
          kind: kind(),
          target: target().trim(),
          args: file ? emptyToNull(args()) : null,
          workingDir: file ? emptyToNull(workingDir()) : null,
          tracking: "default",
          trackingPath: null,
        },
      },
      previewSrc(),
    );
  }

  return (
    <form class="profile__body" onSubmit={save}>
      <label class="field">
        <span class="field__label">{tr("name")}</span>
        <input
          class="input"
          data-autofocus
          value={name()}
          placeholder={tr("toolName")}
          onInput={(event) => setName(event.currentTarget.value)}
        />
      </label>
      <div class="field">
        <span class="field__label" id={typeId}>
          {tr("toolType")}
        </span>
        <div class="segmented" role="radiogroup" aria-labelledby={typeId}>
          <button
            type="button"
            role="radio"
            class="segmented__option"
            aria-checked={kind() === "file"}
            onClick={() => setKind("file")}
          >
            {tr("program")}
          </button>
          <button
            type="button"
            role="radio"
            class="segmented__option"
            aria-checked={kind() === "uri"}
            onClick={() => setKind("uri")}
          >
            {tr("link")}
          </button>
        </div>
      </div>
      <div class="field field--full">
        <label class="field__label" for={targetId}>
          {kind() === "file" ? tr("program") : tr("link")}
        </label>
        <span class="field__row">
          <input
            id={targetId}
            class="input"
            value={target()}
            placeholder={kind() === "uri" ? tr("linkPlaceholder") : undefined}
            onInput={(event) => setTarget(event.currentTarget.value)}
          />
          <Show when={kind() === "file"}>
            <button
              type="button"
              class="btn"
              onClick={() => void pickExecutable().then((path) => path !== null && setTarget(path))}
            >
              {tr("browse")}
            </button>
          </Show>
        </span>
        <Show when={kind() === "file"}>
          <span class="field__hint">{tr("programHint")}</span>
        </Show>
      </div>
      <Show when={kind() === "file"}>
        <label class="field field--full">
          <span class="field__label">{tr("arguments")}</span>
          <input
            class="input"
            value={args()}
            placeholder={tr("argsPlaceholder")}
            onInput={(event) => setArgs(event.currentTarget.value)}
          />
        </label>
        <label class="field field--full">
          <span class="field__label">{tr("workingDir")}</span>
          <span class="field__row">
            <input
              class="input"
              value={workingDir()}
              placeholder={tr("workingDirPlaceholder")}
              onInput={(event) => setWorkingDir(event.currentTarget.value)}
            />
            <button
              type="button"
              class="btn"
              onClick={() =>
                void pickDirectory().then((path) => path !== null && setWorkingDir(path))
              }
            >
              {tr("browse")}
            </button>
          </span>
        </label>
      </Show>
      <div class="field field--full">
        <span class="field__label" id={iconId}>
          {tr("toolIcon")}
        </span>
        <span class="field__row tools-edit__icon" role="group" aria-labelledby={iconId}>
          <span class="tools-edit__icon-preview">
            <ToolIcon kind={kind()} src={previewSrc()} />
          </span>
          <button type="button" class="btn" onClick={() => void chooseIcon()}>
            {tr("chooseToolIcon")}
          </button>
          <button
            type="button"
            class="btn"
            aria-haspopup="dialog"
            onClick={() => setSearching(true)}
          >
            {tr("searchToolIcon")}
          </button>
          <Show when={icon() !== null}>
            <button
              type="button"
              class="btn"
              onClick={() => {
                setIcon(null);
                setPickedSrc(undefined);
              }}
            >
              {tr("defaultToolIcon")}
            </button>
          </Show>
        </span>
        <span class="field__hint">{tr("toolIconHint")}</span>
      </div>
      <Show when={searching()}>
        <Portal>
          {/* Outside the Edit dialog, so its focus trap doesn't fight the picker's. */}
          <ImagePicker
            gameId={null}
            gameName={name().trim() || target().trim()}
            slot="icon"
            current={null}
            onClose={() => setSearching(false)}
            onPick={(url) => {
              setSearching(false);
              void useIcon(url);
            }}
          />
        </Portal>
      </Show>
      <Show when={error()}>
        <p class="error-text field--full" role="alert">
          {error()}
        </p>
      </Show>
      <div class="profile__actions field--full">
        <button type="button" class="btn" onClick={() => props.onCancel()}>
          {tr("cancel")}
        </button>
        <button type="submit" class="btn btn--primary">
          {props.tool ? tr("saveTool") : tr("addTool")}
        </button>
      </div>
    </form>
  );
}

/**
 * The tools of the Launch section: programs and links shown next to Play. A draft like the rest
 * of the Edit dialog (`onChange` gets the edited list). Drag the grip to reorder; Alt+Up/Down on
 * a focused tool moves it too.
 */
export function EditToolsList(props: {
  game: Game;
  tools: GameTool[];
  onChange: (tools: GameTool[]) => void;
}) {
  // The saved tools' pictures; a tool saved in this draft shows what its form showed.
  const saved = createToolIcons(() => props.game);
  const [drafted, setDrafted] = createSignal(new Map<string, string | undefined>());
  const iconSrc = (tool: GameTool): string | undefined =>
    drafted().has(tool.id) ? drafted().get(tool.id) : saved().get(tool.id);
  const [open, setOpen] = createSignal<string | null>(null);
  const [menu, setMenu] = createSignal<{ tool: GameTool; anchor: PopoverAnchor } | null>(null);
  const [dragged, setDragged] = createSignal<number | null>(null);
  const menuButtons = new Map<string, HTMLButtonElement>();
  const nameButtons = new Map<string, HTMLButtonElement>();

  function save(tool: GameTool, src: string | undefined) {
    setDrafted((current) => new Map(current).set(tool.id, src));
    const exists = props.tools.some((t) => t.id === tool.id);
    props.onChange(
      exists ? props.tools.map((t) => (t.id === tool.id ? tool : t)) : [...props.tools, tool],
    );
    setOpen(null);
  }

  function move(index: number, by: -1 | 1) {
    const moved = moveItem(props.tools, index, by);
    props.onChange(moved);
    const id = props.tools[index]?.id;
    if (id !== undefined) queueMicrotask(() => nameButtons.get(id)?.focus());
  }

  return (
    <div class="tools-edit">
      <h3 class="edit-section__title">{tr("tools")}</h3>
      <p class="field__hint">{tr("toolsIntro")}</p>
      <Show when={props.tools.length > 0} fallback={<p class="muted">{tr("noTools")}</p>}>
        <ol class="profiles" aria-label={tr("toolsList")}>
          <For each={props.tools}>
            {(tool, index) => (
              <li
                class="profile"
                classList={{
                  "profile--open": open() === tool.id,
                  "profile--dragging": dragged() === index(),
                }}
                onDragOver={(event) => {
                  if (dragged() !== null) event.preventDefault();
                }}
                onDrop={(event) => {
                  event.preventDefault();
                  const from = dragged();
                  setDragged(null);
                  if (from === null || from === index()) return;
                  const next = [...props.tools];
                  const [item] = next.splice(from, 1);
                  if (item !== undefined) next.splice(index(), 0, item);
                  props.onChange(next);
                }}
              >
                <div class="profile__top">
                  <span
                    class="profile__drag"
                    draggable="true"
                    data-tooltip={tr("dragToReorder")}
                    aria-hidden="true"
                    onDragStart={(event) => {
                      setDragged(index());
                      event.dataTransfer?.setData("text/plain", tool.id);
                    }}
                    onDragEnd={() => setDragged(null)}
                  >
                    <GripVertical class="icon" />
                  </span>
                  <ToolIcon kind={tool.action.kind} src={iconSrc(tool)} />
                  <button
                    ref={(el) => nameButtons.set(tool.id, el)}
                    type="button"
                    class="profile__name"
                    aria-expanded={open() === tool.id}
                    onClick={() => setOpen(open() === tool.id ? null : tool.id)}
                    onKeyDown={(event) => {
                      if (!event.altKey) return;
                      if (event.key === "ArrowUp" && index() > 0) {
                        event.preventDefault();
                        move(index(), -1);
                      } else if (event.key === "ArrowDown" && index() < props.tools.length - 1) {
                        event.preventDefault();
                        move(index(), 1);
                      }
                    }}
                  >
                    {tool.name}
                    <small class="muted tools-edit__target"> {detail(tool)}</small>
                  </button>
                  <button
                    ref={(el) => menuButtons.set(tool.id, el)}
                    type="button"
                    class="icon-btn"
                    aria-label={tr("actions", { name: tool.name })}
                    aria-haspopup="menu"
                    onClick={(event) => {
                      const rect = event.currentTarget.getBoundingClientRect();
                      setMenu(menu() ? null : { tool, anchor: { rect, align: "end" } });
                    }}
                  >
                    <Ellipsis class="icon" aria-hidden="true" />
                  </button>
                </div>
                <Show when={open() === tool.id}>
                  <ToolForm
                    gameId={props.game.id}
                    tool={tool}
                    iconSrc={iconSrc(tool)}
                    onSave={save}
                    onCancel={() => setOpen(null)}
                  />
                </Show>
              </li>
            )}
          </For>
        </ol>
      </Show>
      <Show
        when={open() === "new"}
        fallback={
          <button type="button" class="btn profiles__add" onClick={() => setOpen("new")}>
            <Plus class="icon" aria-hidden="true" /> {tr("addTool")}
          </button>
        }
      >
        <div class="profile profile--open">
          <ToolForm
            gameId={props.game.id}
            tool={null}
            iconSrc={undefined}
            onSave={save}
            onCancel={() => setOpen(null)}
          />
        </div>
      </Show>
      <Show when={menu()}>
        {(current) => {
          // Read once: the menu closes (and `menu()` is gone) before the chosen entry runs.
          const { tool, anchor } = current();
          return (
            <Menu
              anchor={anchor}
              label={tr("actions", { name: tool.name })}
              inside={() => [menuButtons.get(tool.id)]}
              onClose={(returnFocus) => {
                const button = menuButtons.get(tool.id);
                setMenu(null);
                if (returnFocus) button?.focus();
              }}
              entries={[
                { label: tr("edit"), onSelect: () => setOpen(tool.id) },
                {
                  label: tr("duplicate"),
                  onSelect: () =>
                    props.onChange([
                      ...props.tools,
                      {
                        ...tool,
                        id: crypto.randomUUID(),
                        name: tr("copyName", { name: tool.name }),
                      },
                    ]),
                },
                { kind: "separator" },
                {
                  label: tr("delete"),
                  danger: true,
                  onSelect: () => props.onChange(props.tools.filter((t) => t.id !== tool.id)),
                },
              ]}
            />
          );
        }}
      </Show>
    </div>
  );
}
