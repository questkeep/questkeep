import type { ActivityActorFilter } from "../bindings/ActivityActorFilter";
import type { ActivityBatch } from "../bindings/ActivityBatch";
import type { ActivityChange } from "../bindings/ActivityChange";
import type { ActivityCursor } from "../bindings/ActivityCursor";
import type { ActivityEntry } from "../bindings/ActivityEntry";
import type { ActivityItem } from "../bindings/ActivityItem";
import type { ActivityKind } from "../bindings/ActivityKind";
import type { ActivityQuery } from "../bindings/ActivityQuery";
import type { Actor } from "../bindings/Actor";
import type { SystemEvent } from "../bindings/SystemEvent";
import type { SystemLevel } from "../bindings/SystemLevel";
import type { SystemQuery } from "../bindings/SystemQuery";
import { intlLocale, translator } from "../i18n";
import { activity } from "../i18n/activity";
import { formatPlaytime, formatReleaseDate, formatSize } from "./format";

const tr = translator(activity);

export type EntryItem = Extract<ActivityItem, { type: "entry" }>;
export type BatchItem = Extract<ActivityItem, { type: "batch" }>;
export type SessionItem = Extract<ActivityItem, { type: "session" }>;

/** How ids become names: stores, plugins and completion statuses. */
export interface ActivityNames {
  store: (id: string) => string;
  plugin: (id: string) => string;
  /** A completion status's name, or null when it no longer exists. */
  status: (id: string) => string | null;
}

/** Every "What" in filter order. */
export const ACTIVITY_KINDS: readonly ActivityKind[] = [
  "added",
  "removed",
  "metadata",
  "edit",
  "images",
  "install",
  "combine",
  "categories",
  "status",
  "sessions",
  "profiles",
  "scripts",
  "other",
];

/** System event kinds the "Kind" filter offers. */
export const SYSTEM_KINDS = [
  "sync",
  "syncFailed",
  "signInExpired",
  "backup",
  "backupFailed",
  "restore",
  "restoreFailed",
  "updateCheck",
  "updateCheckFailed",
  "notification",
  "pluginInstalled",
  "pluginUpdated",
  "pluginEnabled",
  "pluginDisabled",
  "pluginReloaded",
  "pluginFailed",
  "pluginRemoved",
  "http",
  "log",
] as const;
export type SystemKind = (typeof SYSTEM_KINDS)[number];

const args = (value: Record<string, unknown> | null | undefined) => value ?? {};
const text = (value: unknown): string | null =>
  typeof value === "string" && value.trim() !== "" ? value : null;
const num = (value: unknown): number | null =>
  typeof value === "number" && Number.isFinite(value) ? value : null;

// ---------------------------------------------------------------------------------------------
// Who

/** A key for an actor, also the value of the "Who" filter: `user`, `app`, `store:x`, `plugin:x`. */
export function actorKey(actor: Actor): string {
  return actor.kind === "store" || actor.kind === "plugin"
    ? `${actor.kind}:${actor.id ?? ""}`
    : actor.kind;
}

/** "You", "Questkeep", "Steam", "BSManager plugin". */
export function actorLabel(actor: Actor, names: ActivityNames): string {
  switch (actor.kind) {
    case "user":
      return tr("you");
    case "app":
      return tr("questkeep");
    case "store":
      return actor.id ? names.store(actor.id) : tr("stores");
    case "plugin":
      return tr("pluginActor", { name: actor.id ? names.plugin(actor.id) : tr("plugins") });
  }
}

/** A whole sentence for screen readers and plain text: "You refreshed metadata". */
export function sentence(actor: string, action: string): string {
  return tr("sentence", { actor, action });
}

// ---------------------------------------------------------------------------------------------
// Why: entry and batch summaries

const SYNC_REASONS = new Set(["sync", "autoSync"]);

/** The trigger of a sync as a short note ("automatic, at start"), or null for a plain sync. */
export function triggerText(trigger: unknown): string | null {
  switch (trigger) {
    case "signIn":
      return tr("t_signIn");
    case "addGame":
      return tr("t_addGame");
    case "install":
      return tr("t_install");
    case "startup":
      return tr("t_startup");
    case "auto":
      return tr("t_auto");
    case "installFolder":
      return tr("t_installFolder");
    default:
      return null;
  }
}

/** A metadata source's name: IGDB, SteamGridDB, or a store. */
export function sourceLabel(id: string, names: ActivityNames): string {
  if (id === "igdb") return "IGDB";
  if (id === "steamgriddb") return "SteamGridDB";
  return names.store(id);
}

/** A date stored in the args (the restored backup's), in the local short format. */
function dateText(value: unknown): string | null {
  const iso = text(value);
  if (!iso) return null;
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? null : date.toLocaleString(intlLocale());
}

type Why = Pick<ActivityEntry, "reason" | "args" | "kind" | "batchId">;

/** What was done, without the actor ("Refreshed metadata", "Created the category “VR”"). */
export function entryAction(entry: Why, names: ActivityNames): string {
  const a = args(entry.args);
  const name = text(a.name) ?? "";
  const to = text(a.to) ?? "";
  if (SYNC_REASONS.has(entry.reason)) {
    switch (entry.kind) {
      case "added":
        return tr("r_addedSync");
      case "removed":
        return tr("r_removedSync");
      case "install":
        return tr("r_installSync");
      default:
        return tr("r_updatedSync");
    }
  }
  switch (entry.reason) {
    case "metadataRefresh":
      return tr("r_metadataRefresh");
    case "metadataFetch":
      return tr("r_metadataFetch");
    case "artworkFetch":
      return tr("r_artworkFetch");
    case "artworkRefetch":
      return tr("r_artworkRefetch");
    case "resolveAll":
      return tr("r_resolveAll");
    case "edit":
      switch (entry.kind) {
        case "images":
          return tr("r_editImages");
        case "status":
          return tr("r_editStatus");
        case "categories":
          return tr("r_editCategories");
        case "install":
          return tr("r_editInstall");
        default:
          return tr("r_edit");
      }
    case "combine":
      return tr("r_combine");
    case "separate":
      return tr("r_separate");
    case "preferCopy":
      return tr("r_preferCopy");
    case "bulkChange":
      return tr("r_bulkChange");
    case "remove":
      return tr("r_remove");
    case "addManual":
      return tr("r_addManual");
    case "metadataLink":
      return tr("r_metadataLink");
    case "sessionEdit":
      return tr("r_sessionEdit");
    case "completionStatus":
      return tr("r_completionStatus");
    case "completionSettings":
      return tr("r_completionSettings");
    case "firstStart": {
      const id = text(a.status);
      const status = id ? (names.status(id) ?? id) : null;
      return status ? tr("r_firstStartTo", { status }) : tr("r_firstStart");
    }
    case "categoryCreate":
      return tr("r_categoryCreate", { name });
    case "categoryRename":
      return tr("r_categoryRename", { name, to });
    case "categoryDelete":
      return tr("r_categoryDelete", { name });
    case "categoryMerge":
      return tr("r_categoryMerge", { name, to });
    case "categorySeparate":
      return tr("r_categorySeparate", { name, to });
    case "categoryFilter":
      return tr(a.filter === false ? "r_categoryFilterOff" : "r_categoryFilterOn", { name });
    case "setCategories":
      return tr("r_setCategories");
    case "autoCategories":
      return tr("r_autoCategories");
    case "launchProfiles":
      return tr("r_launchProfiles");
    case "scriptSave":
      return tr("r_scriptSave", { name });
    case "scriptDelete":
      return tr("r_scriptDelete", { name });
    case "scriptOrder":
      return tr("r_scriptOrder");
    case "restoreBackup": {
      const date = dateText(a.createdAt);
      return date ? tr("r_restoreBackup", { date }) : tr("r_restoreBackupPlain");
    }
    default:
      return tr("r_other");
  }
}

/** The small note after the action ("from IGDB", "automatic, at start"), if any. */
export function entryDetail(entry: Why, names: ActivityNames): string | null {
  const a = args(entry.args);
  if (SYNC_REASONS.has(entry.reason)) {
    const trigger = triggerText(a.trigger);
    return trigger ? `${tr("r_duringSync")}, ${trigger}` : tr("r_duringSync");
  }
  if (entry.reason === "metadataRefresh" || entry.reason === "metadataFetch") {
    const list = Array.isArray(a.sources) ? a.sources : text(a.source) ? [a.source] : [];
    const sources = list.flatMap((s) => (typeof s === "string" ? [sourceLabel(s, names)] : []));
    if (sources.length > 0) return tr("r_fromSources", { sources: sources.join(", ") });
  }
  return null;
}

/** What a batch did ("Synced", "Changed several games at once"). */
export function batchAction(batch: Pick<ActivityBatch, "reason" | "args">, names: ActivityNames) {
  if (SYNC_REASONS.has(batch.reason)) return tr("r_synced");
  return entryAction(
    { reason: batch.reason, args: batch.args, kind: "other", batchId: null },
    names,
  );
}

/** A batch's note: the sync trigger, the metadata sources. */
export function batchDetail(
  batch: Pick<ActivityBatch, "reason" | "args">,
  names: ActivityNames,
): string | null {
  if (SYNC_REASONS.has(batch.reason)) return triggerText(args(batch.args).trigger);
  return entryDetail(
    { reason: batch.reason, args: batch.args, kind: "other", batchId: null },
    names,
  );
}

const COUNT_KEYS = [
  ["added", "countAdded"],
  ["updated", "countUpdated"],
  ["removed", "countRemoved"],
  ["uninstalled", "countUninstalled"],
] as const;

/** "3 added · 12 updated" for a sync, else "4 games". */
export function batchCounts(batch: Pick<ActivityBatch, "counts" | "gameCount">): string {
  const parts: string[] = COUNT_KEYS.flatMap(([key, label]) => {
    const count = batch.counts[key] ?? 0;
    return count > 0 ? [tr(label, { count })] : [];
  });
  const playtime = batch.counts.playtime ?? 0;
  if (playtime > 0) parts.push(tr("countPlaytime", { count: playtime }));
  if (parts.length === 0 && batch.gameCount > 0)
    parts.push(tr("games", { count: batch.gameCount }));
  return parts.join(" · ");
}

/** "3 fields" (nothing for none). */
export function changeCountText(count: number): string {
  return count > 0 ? tr("fields", { count }) : "";
}

/** "Ran for 1h 12m". */
export function sessionText(seconds: number): string {
  return tr("ranFor", { time: formatPlaytime(seconds) });
}

/** One line of text for any item, with its actor: the Recent activity card, screen readers. */
export function itemSentence(item: ActivityItem, names: ActivityNames): string {
  switch (item.type) {
    case "session":
      return sessionText(item.durationSeconds);
    case "batch":
      return sentence(actorLabel(item.actor, names), batchAction(item, names));
    case "entry":
      return sentence(actorLabel(item.actor, names), entryAction(item, names));
  }
}

// ---------------------------------------------------------------------------------------------
// What: field labels and change values

const FIELD_KEYS = {
  name: "f_name",
  sortingName: "f_sortingName",
  kind: "f_kind",
  description: "f_description",
  releaseDate: "f_releaseDate",
  developers: "f_developers",
  publishers: "f_publishers",
  genres: "f_genres",
  tags: "f_tags",
  features: "f_features",
  series: "f_series",
  ageRatings: "f_ageRatings",
  platforms: "f_platforms",
  gameModes: "f_gameModes",
  themes: "f_themes",
  perspectives: "f_perspectives",
  criticScore: "f_criticScore",
  communityScore: "f_communityScore",
  timeToBeat: "f_timeToBeat",
  installSize: "f_installSize",
  links: "f_links",
  completionStatus: "f_completionStatus",
  userScore: "f_userScore",
  notes: "f_notes",
  favorite: "f_favorite",
  hidden: "f_hidden",
  installed: "f_installed",
  installDir: "f_installDir",
  playAction: "f_playAction",
  detectLaunches: "f_detectLaunches",
  ignoredProcesses: "f_ignoredProcesses",
  categories: "f_categories",
  copies: "f_copies",
  preferredCopy: "f_preferredCopy",
  launchProfiles: "f_launchProfiles",
  defaultProfile: "f_defaultProfile",
  metadataLink: "f_metadataLink",
  metadataName: "f_metadataName",
  cover: "f_cover",
  background: "f_background",
  icon: "f_icon",
  combinedInto: "f_combinedInto",
  runGlobalScripts: "f_runGlobalScripts",
  session: "f_session",
} as const;

/** A field key's label ("genres" → "Genres"); an unknown key is shown as it is. */
export function fieldLabel(field: string): string {
  const key = (FIELD_KEYS as Record<string, (typeof FIELD_KEYS)[keyof typeof FIELD_KEYS]>)[field];
  return key ? tr(key) : field;
}

/** Long texts (descriptions, notes) are cut in the table; the full text is in the tooltip. */
export const MAX_VALUE_LENGTH = 160;

function clip(value: string): string {
  const flat = value.replace(/\s+/g, " ").trim();
  return flat.length > MAX_VALUE_LENGTH ? `${flat.slice(0, MAX_VALUE_LENGTH - 1)}…` : flat;
}

const BOOLEAN_FIELDS = new Set([
  "favorite",
  "hidden",
  "installed",
  "detectLaunches",
  "runGlobalScripts",
]);

function record(value: unknown): Record<string, unknown> | null {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

/** "Steam · 1.40.0 (620980)". */
function copyText(value: Record<string, unknown>, names: ActivityNames): string {
  const store = text(value.store);
  const qualifier = text(value.qualifier);
  const id = text(value.storeId);
  const base = [store ? names.store(store) : null, qualifier].filter((p) => p !== null).join(" · ");
  return id ? `${base} (${id})` : base;
}

/** One value of a field, ready to show; null for none. */
export function valueText(field: string, value: unknown, names: ActivityNames): string | null {
  if (value === null || value === undefined || value === "") return null;
  if (BOOLEAN_FIELDS.has(field) || typeof value === "boolean") {
    return value === true ? tr("yes") : tr("no");
  }
  const obj = record(value);
  switch (field) {
    case "releaseDate":
      return typeof value === "string" ? formatReleaseDate(value) : null;
    case "kind":
      return value === "app" ? tr("kindApp") : value === "game" ? tr("kindGame") : String(value);
    case "completionStatus":
      return typeof value === "string" ? (names.status(value) ?? value) : null;
    case "installSize":
      return num(value) === null ? null : formatSize(value as number);
    case "criticScore":
    case "communityScore": {
      const score = obj ? num(obj.value) : num(value);
      if (score === null) return null;
      const source = obj ? text(obj.source) : null;
      return source ? `${score} (${source})` : String(score);
    }
    case "timeToBeat": {
      if (!obj) return null;
      const parts = (
        [
          ["main", "ttbMain"],
          ["extra", "ttbExtra"],
          ["complete", "ttbComplete"],
        ] as const
      ).flatMap(([key, label]) => {
        const seconds = num(obj[key]);
        return seconds && seconds > 0 ? [tr(label, { time: formatPlaytime(seconds) })] : [];
      });
      return parts.length > 0 ? parts.join(" · ") : null;
    }
    case "playAction": {
      if (!obj) return null;
      const target = text(obj.target) ?? "";
      const extra = text(obj.args);
      return extra ? `${target} ${extra}` : target;
    }
    case "preferredCopy":
    case "copies":
      return obj ? copyText(obj, names) : String(value);
    case "links": {
      if (!obj) return String(value);
      const label = text(obj.name);
      const url = text(obj.url) ?? "";
      return label && label !== url ? `${label} (${url})` : url;
    }
    case "launchProfiles": {
      if (!obj) return String(value);
      const label = text(obj.name) ?? "";
      const extra = text(obj.args);
      return extra ? `${label} (${extra})` : label;
    }
    case "metadataLink": {
      if (!obj) return typeof value === "string" ? value : null;
      const label = text(obj.name) ?? text(obj.id) ?? "";
      const source = text(obj.source);
      return source ? `${label} (${sourceLabel(source, names)})` : label;
    }
    case "session": {
      if (!obj) return null;
      const started = text(obj.startedAt);
      const seconds = num(obj.durationSeconds) ?? 0;
      const date = started ? new Date(started).toLocaleString(intlLocale()) : "";
      return tr("sessionValue", { date, time: formatPlaytime(seconds) });
    }
    default:
      break;
  }
  if (typeof value === "string") return clip(value);
  if (typeof value === "number") return value.toLocaleString(intlLocale());
  if (obj) return clip(text(obj.name) ?? JSON.stringify(obj));
  return clip(JSON.stringify(value));
}

/** How a change row shows: before → after, added/removed items, or a word for images. */
export type ChangeView =
  | { mode: "value"; field: string; label: string; before: string | null; after: string | null }
  | { mode: "set"; field: string; label: string; added: string[]; removed: string[] }
  | { mode: "changed"; field: string; label: string; text: string };

const listOf = (value: unknown): unknown[] => (Array.isArray(value) ? value : []);

/** A change as the details table shows it. */
export function changeView(change: ActivityChange, names: ActivityNames): ChangeView {
  const label = fieldLabel(change.field);
  const show = (v: unknown) => valueText(change.field, v, names) ?? "";
  switch (change.mode) {
    case "set":
      return {
        mode: "set",
        field: change.field,
        label,
        removed: listOf(change.before)
          .map(show)
          .filter((s) => s !== ""),
        added: listOf(change.after)
          .map(show)
          .filter((s) => s !== ""),
      };
    case "changed": {
      const had = change.before === true;
      const has = change.after === true;
      const word = !had && has ? "imageAdded" : had && !has ? "imageRemoved" : "imageChanged";
      return { mode: "changed", field: change.field, label, text: tr(word) };
    }
    case "value":
      return {
        mode: "value",
        field: change.field,
        label,
        before: valueText(change.field, change.before, names),
        after: valueText(change.field, change.after, names),
      };
  }
}

/** The full text of a long value, for its tooltip (null when it fits). */
export function fullValue(value: unknown): string | null {
  return typeof value === "string" && value.replace(/\s+/g, " ").trim().length > MAX_VALUE_LENGTH
    ? value
    : null;
}

// ---------------------------------------------------------------------------------------------
// Days

/** When an item happened (an entry's time, a batch's start, a session's start). */
export function itemAt(item: ActivityItem): string {
  switch (item.type) {
    case "entry":
      return item.at;
    case "batch":
      return item.startedAt;
    case "session":
      return item.startedAt;
  }
}

/** A stable key per item (types share id ranges). */
export function itemKey(item: ActivityItem): string {
  return `${item.type}:${item.id}`;
}

function dayStart(date: Date): number {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
}

/** "Today", "Yesterday", or the date ("12 Nov 2026"). */
export function dayLabel(iso: string, now: Date): string {
  const date = new Date(iso);
  const days = Math.round((dayStart(now) - dayStart(date)) / 86_400_000);
  if (days === 0) return tr("today");
  if (days === 1) return tr("yesterday");
  return date.toLocaleDateString(intlLocale(), {
    day: "numeric",
    month: "short",
    year: "numeric",
  });
}

export interface DayGroup<T> {
  /** The local date, `YYYY-M-D`. */
  key: string;
  label: string;
  items: T[];
}

/** Splits items (newest first) into local days, keeping their order. */
export function groupByDay<T>(
  items: readonly T[],
  at: (item: T) => string,
  now: Date,
): DayGroup<T>[] {
  const groups: DayGroup<T>[] = [];
  for (const item of items) {
    const iso = at(item);
    const date = new Date(iso);
    const key = `${date.getFullYear()}-${date.getMonth() + 1}-${date.getDate()}`;
    const last = groups.at(-1);
    if (last?.key === key) last.items.push(item);
    else groups.push({ key, label: dayLabel(iso, now), items: [item] });
  }
  return groups;
}

/** Items of the next page appended to those shown, without repeating any. */
export function appendPage<T>(
  shown: readonly T[],
  page: readonly T[],
  key: (item: T) => string,
): T[] {
  const seen = new Set(shown.map(key));
  return [...shown, ...page.filter((item) => !seen.has(key(item)))];
}

// ---------------------------------------------------------------------------------------------
// Filters → queries

/** "everyone", "user", "app", "store:<id>" or "plugin:<id>". */
export type Who = string;
export type When = "all" | "today" | "7d" | "30d" | "90d";
export const WHEN_OPTIONS: readonly When[] = ["all", "today", "7d", "30d", "90d"];

/** The "Who" filter as the query's actor list. */
export function whoActors(who: Who): ActivityActorFilter[] {
  if (who === "user" || who === "app") return [{ kind: who }];
  const [kind, ...rest] = who.split(":");
  const id = rest.join(":");
  if ((kind === "store" || kind === "plugin") && id !== "") return [{ kind, id }];
  return [];
}

/** The start of a "When" range, or undefined for any time. */
export function whenFrom(when: When, now: Date): string | undefined {
  const day = 86_400_000;
  switch (when) {
    case "all":
      return undefined;
    case "today":
      return new Date(dayStart(now)).toISOString();
    case "7d":
      return new Date(now.getTime() - 7 * day).toISOString();
    case "30d":
      return new Date(now.getTime() - 30 * day).toISOString();
    case "90d":
      return new Date(now.getTime() - 90 * day).toISOString();
  }
}

export interface LibraryFilter {
  /** Part of a game's name. */
  search: string;
  who: Who;
  what: ActivityKind | "all";
  when: When;
  sessions: boolean;
  /** One game's log (the drawer, "Open in the Activity page"). */
  gameId: string | null;
  /** One batch's entries (a sync toast's "Details"). */
  batchId: number | null;
}

export const DEFAULT_LIBRARY_FILTER: LibraryFilter = {
  search: "",
  who: "everyone",
  what: "all",
  when: "all",
  sessions: true,
  gameId: null,
  batchId: null,
};

export const PAGE_SIZE = 50;

/** The query for a page of the library part. */
export function libraryQuery(
  filter: LibraryFilter,
  now: Date,
  before?: ActivityCursor | null,
  limit = PAGE_SIZE,
): ActivityQuery {
  const query: ActivityQuery = {
    actors: whoActors(filter.who),
    kinds: filter.what === "all" ? [] : [filter.what],
    includeSessions: filter.sessions && filter.batchId === null,
    limit,
  };
  const from = whenFrom(filter.when, now);
  if (from) query.from = from;
  const search = filter.search.trim();
  if (search !== "") query.search = search;
  if (filter.gameId !== null) query.gameId = filter.gameId;
  if (filter.batchId !== null) query.batchId = filter.batchId;
  if (before) query.before = before;
  return query;
}

/** Whether any filter narrows the list (to tell "nothing yet" from "nothing matches"). */
export function isFiltered(filter: LibraryFilter): boolean {
  return (
    filter.search.trim() !== "" ||
    filter.who !== "everyone" ||
    filter.what !== "all" ||
    filter.when !== "all" ||
    filter.gameId !== null ||
    filter.batchId !== null
  );
}

export interface SystemFilter {
  search: string;
  /** "all", "app", "store", "plugin", "store:<id>" or "plugin:<id>". */
  source: string;
  level: SystemLevel;
  kind: SystemKind | "all";
  when: When;
}

export const DEFAULT_SYSTEM_FILTER: SystemFilter = {
  search: "",
  source: "all",
  level: "info",
  kind: "all",
  when: "all",
};

/** The query for a page of the system part. */
export function systemQuery(
  filter: SystemFilter,
  now: Date,
  before?: ActivityCursor | null,
  limit = PAGE_SIZE,
): SystemQuery {
  const query: SystemQuery = {
    sources: filter.source === "all" ? [] : [filter.source],
    kinds: filter.kind === "all" ? [] : [filter.kind],
    limit,
  };
  if (filter.level !== "info") query.minLevel = filter.level;
  const from = whenFrom(filter.when, now);
  if (from) query.from = from;
  const search = filter.search.trim();
  if (search !== "") query.search = search;
  if (before) query.before = before;
  return query;
}

export function isSystemFiltered(filter: SystemFilter): boolean {
  return (
    filter.search.trim() !== "" ||
    filter.source !== "all" ||
    filter.level !== "info" ||
    filter.kind !== "all" ||
    filter.when !== "all"
  );
}

// ---------------------------------------------------------------------------------------------
// System events

/** Who a system event is from, as an actor (Questkeep, a store, a plugin). */
export function systemActor(event: Pick<SystemEvent, "sourceKind" | "sourceId">): Actor {
  if (event.sourceKind === "store" && event.sourceId) return { kind: "store", id: event.sourceId };
  if (event.sourceKind === "plugin" && event.sourceId)
    return { kind: "plugin", id: event.sourceId };
  return { kind: "app" };
}

/** The label of a system kind for the "Kind" filter. */
export function systemKindLabel(kind: SystemKind): string {
  return tr(`sk_${kind}`);
}

/** A system event's line, translated from its details where the kind is known. */
export function systemMessage(event: SystemEvent, names: ActivityNames): string {
  const d = event.details ?? {};
  switch (event.kind) {
    case "sync":
      return tr("m_sync", { added: num(d.added) ?? 0, updated: num(d.updated) ?? 0 });
    case "backup": {
      const size = num(d.size);
      return size === null ? tr("m_backup") : tr("m_backupSize", { size: formatSize(size) });
    }
    case "restore":
      return tr("m_restore");
    case "updateCheck": {
      const version = text(d.version);
      return version ? tr("m_updateAvailable", { version }) : tr("m_upToDate");
    }
    case "signInExpired":
      return tr("m_signInExpired");
    case "pluginUpdated": {
      const from = text(d.from);
      const to = text(d.to);
      return from && to ? tr("m_pluginUpdated", { from, to }) : event.message;
    }
    case "pluginEnabled":
      return tr("m_pluginEnabled");
    case "pluginDisabled":
      return tr("m_pluginDisabled");
    case "pluginReloaded":
      return tr("m_pluginReloaded");
    case "pluginInstalled": {
      const version = text(d.version);
      return version ? tr("m_pluginInstalled", { version }) : tr("m_pluginInstalledPlain");
    }
    case "pluginRemoved":
      return tr("m_pluginRemoved");
    case "http": {
      const host = text(d.host);
      const count = num(d.requests);
      if (!host || count === null) return event.message;
      const parts = [tr("m_http", { count, host })];
      const failed = num(d.failed) ?? 0;
      if (failed > 0) parts.push(tr("m_httpFailed", { failed }));
      const bytes = num(d.bytes) ?? 0;
      if (bytes > 0) parts.push(tr("m_received", { size: formatSize(bytes) }));
      return parts.join(" · ");
    }
    case "notification": {
      const title = text(d.title);
      return title && !event.message.startsWith(title)
        ? `${title}: ${event.message}`
        : event.message;
    }
    default:
      void names;
      return event.message;
  }
}

/** The details of a system event as label/value lines (for the expanded row). */
export function systemDetails(event: SystemEvent): { key: string; value: string }[] {
  return Object.entries(event.details ?? {}).map(([key, value]) => ({
    key,
    value: typeof value === "string" ? value : JSON.stringify(value),
  }));
}

/** The label of a system level. */
export function levelLabel(level: SystemLevel): string {
  return tr(level === "info" ? "lv_info" : level === "warn" ? "lv_warn" : "lv_error");
}
