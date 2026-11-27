import { createUniqueId, For, Show } from "solid-js";
import type { ActivityKind } from "../bindings/ActivityKind";
import type { SystemLevel } from "../bindings/SystemLevel";
import { translator } from "../i18n";
import { activity as dict } from "../i18n/activity";
import {
  ACTIVITY_KINDS,
  SYSTEM_KINDS,
  systemKindLabel,
  WHEN_OPTIONS,
  type LibraryFilter,
  type SystemFilter,
  type SystemKind,
  type When,
} from "../lib/activity";
import { store } from "../lib/store";
import { Select, type SelectItem } from "./Select";

const tr = translator(dict);

const WHEN_LABELS = {
  all: "whenAll",
  today: "whenToday",
  "7d": "when7",
  "30d": "when30",
  "90d": "when90",
} as const;

/** Built-in stores (stores of plugins are listed as their plugin, which records their syncs). */
function builtinStores() {
  const pluginStores = new Set(store.plugins().flatMap((p) => p.providers));
  return store.providers().filter((p) => p.id !== "manual" && !pluginStores.has(p.id));
}

/** The "Who" choices: everyone, you, Questkeep, each store, each plugin. */
export function whoOptions(): SelectItem<string>[] {
  const stores = builtinStores();
  const plugins = store.plugins();
  return [
    { value: "everyone", label: tr("everyone") },
    { value: "user", label: tr("you") },
    { value: "app", label: tr("questkeep") },
    ...(stores.length > 0
      ? [
          {
            group: tr("stores"),
            options: stores.map((p) => ({ value: `store:${p.id}`, label: p.name })),
          },
        ]
      : []),
    ...(plugins.length > 0
      ? [
          {
            group: tr("plugins"),
            options: plugins.map((p) => ({ value: `plugin:${p.id}`, label: p.name })),
          },
        ]
      : []),
  ];
}

/** Keeps a chosen store or plugin that is gone selectable (its id as the label). */
function withCurrent(options: SelectItem<string>[], value: string): SelectItem<string>[] {
  const all = options.flatMap((o) => ("group" in o ? o.options : [o]));
  if (all.some((o) => o.value === value)) return options;
  return [...options, { value, label: value.split(":").slice(1).join(":") || value }];
}

const whatOptions = (): SelectItem<ActivityKind | "all">[] => [
  { value: "all", label: tr("allChanges") },
  ...ACTIVITY_KINDS.map((kind) => ({ value: kind, label: tr(`kind_${kind}`) })),
];

const whenOptions = (): SelectItem<When>[] =>
  WHEN_OPTIONS.map((when) => ({ value: when, label: tr(WHEN_LABELS[when]) }));

/** A labeled dropdown in a filter bar. */
function Field<T extends string>(props: {
  label: string;
  value: T;
  options: SelectItem<T>[];
  onChange: (value: T) => void;
}) {
  const id = createUniqueId();
  return (
    <span class="alog-filter">
      <label class="alog-filter__label" id={id}>
        {props.label}
      </label>
      <Select
        class="input alog-filter__select"
        aria-labelledby={id}
        value={props.value}
        options={props.options}
        onChange={props.onChange}
      />
    </span>
  );
}

/** A search box that reports what is typed after a short pause. */
function Search(props: { value: string; placeholder: string; onChange: (text: string) => void }) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  return (
    <input
      type="search"
      class="input alog-search"
      placeholder={props.placeholder}
      aria-label={props.placeholder}
      value={props.value}
      onInput={(event) => {
        const text = event.currentTarget.value;
        clearTimeout(timer);
        timer = setTimeout(() => props.onChange(text), 250);
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape" && event.currentTarget.value !== "") {
          event.stopPropagation();
          event.currentTarget.value = "";
          clearTimeout(timer);
          props.onChange("");
        }
      }}
    />
  );
}

/** The filters of the library part: the page shows all, the drawer leaves out search and When. */
export function LibraryFilters(props: {
  filter: LibraryFilter;
  onChange: (patch: Partial<LibraryFilter>) => void;
  compact?: boolean;
}) {
  return (
    <div class="alog-filters" role="group" aria-label={tr("filters")}>
      <Show when={!props.compact}>
        <Search
          value={props.filter.search}
          placeholder={tr("searchGames")}
          onChange={(search) => props.onChange({ search })}
        />
      </Show>
      <Field
        label={tr("who")}
        value={props.filter.who}
        options={withCurrent(whoOptions(), props.filter.who)}
        onChange={(who) => props.onChange({ who })}
      />
      <Field
        label={tr("what")}
        value={props.filter.what}
        options={whatOptions()}
        onChange={(what) => props.onChange({ what })}
      />
      <Show when={!props.compact}>
        <Field
          label={tr("when")}
          value={props.filter.when}
          options={whenOptions()}
          onChange={(when) => props.onChange({ when })}
        />
      </Show>
      <Show when={props.filter.batchId === null}>
        <label class="check check--switch alog-switch">
          <input
            type="checkbox"
            role="switch"
            checked={props.filter.sessions}
            onChange={(event) => props.onChange({ sessions: event.currentTarget.checked })}
          />
          <span>{tr("sessions")}</span>
        </label>
      </Show>
    </div>
  );
}

const LEVELS: { value: SystemLevel; label: "levelAll" | "levelWarn" | "levelErrors" }[] = [
  { value: "info", label: "levelAll" },
  { value: "warn", label: "levelWarn" },
  { value: "error", label: "levelErrors" },
];

/** The "Source" choices of the system part. */
function sourceOptions(): SelectItem<string>[] {
  const stores = store.providers().filter((p) => p.id !== "manual");
  const plugins = store.plugins();
  return [
    { value: "all", label: tr("sourceAll") },
    { value: "app", label: tr("questkeep") },
    {
      group: tr("stores"),
      options: [
        { value: "store", label: tr("sourceStores") },
        ...stores.map((p) => ({ value: `store:${p.id}`, label: p.name })),
      ],
    },
    {
      group: tr("plugins"),
      options: [
        { value: "plugin", label: tr("sourcePlugins") },
        ...plugins.map((p) => ({ value: `plugin:${p.id}`, label: p.name })),
      ],
    },
  ];
}

/** The filters of the system part. */
export function SystemFilters(props: {
  filter: SystemFilter;
  onChange: (patch: Partial<SystemFilter>) => void;
}) {
  return (
    <div class="alog-filters" role="group" aria-label={tr("filters")}>
      <Search
        value={props.filter.search}
        placeholder={tr("searchMessages")}
        onChange={(search) => props.onChange({ search })}
      />
      <Field
        label={tr("source")}
        value={props.filter.source}
        options={withCurrent(sourceOptions(), props.filter.source)}
        onChange={(source) => props.onChange({ source })}
      />
      <Field
        label={tr("level")}
        value={props.filter.level}
        options={LEVELS.map((l) => ({ value: l.value, label: tr(l.label) }))}
        onChange={(level) => props.onChange({ level })}
      />
      <Field<SystemKind | "all">
        label={tr("kind")}
        value={props.filter.kind}
        options={[
          { value: "all", label: tr("kindAll") },
          ...SYSTEM_KINDS.map((kind) => ({ value: kind, label: systemKindLabel(kind) })),
        ]}
        onChange={(kind) => props.onChange({ kind })}
      />
      <Field
        label={tr("when")}
        value={props.filter.when}
        options={whenOptions()}
        onChange={(when) => props.onChange({ when })}
      />
    </div>
  );
}

/** A filter that narrows the log to one game or one batch, with a button to drop it. */
export function ScopePills(props: { pills: { label: string; onRemove: () => void }[] }) {
  return (
    <Show when={props.pills.length > 0}>
      <div class="alog-scopes">
        <For each={props.pills}>
          {(pill) => (
            <span class="alog-scope">
              {pill.label}
              <button
                type="button"
                class="icon-btn icon-btn--small alog-scope__remove"
                aria-label={tr("removeFilter", { label: pill.label })}
                data-tooltip={tr("removeFilter", { label: pill.label })}
                onClick={() => pill.onRemove()}
              >
                ×
              </button>
            </span>
          )}
        </For>
      </div>
    </Show>
  );
}
