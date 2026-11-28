import { describe, expect, it } from "vitest";
import type { ActivityChange } from "../bindings/ActivityChange";
import type { ActivityItem } from "../bindings/ActivityItem";
import type { ActivityKind } from "../bindings/ActivityKind";
import type { SystemEvent } from "../bindings/SystemEvent";
import {
  ACTIVITY_KINDS,
  actorKey,
  actorLabel,
  appendPage,
  batchAction,
  batchCounts,
  batchDetail,
  changeCountText,
  changeView,
  DEFAULT_LIBRARY_FILTER,
  DEFAULT_SYSTEM_FILTER,
  dayLabel,
  entryAction,
  entryDetail,
  fieldLabel,
  fieldsText,
  fullValue,
  groupByDay,
  isFiltered,
  isSystemFiltered,
  itemAt,
  itemKey,
  itemSentence,
  levelLabel,
  nestedAction,
  libraryQuery,
  MAX_VALUE_LENGTH,
  sessionText,
  sourceLabel,
  SYSTEM_KINDS,
  systemActor,
  systemDetails,
  systemKindLabel,
  systemMessage,
  systemQuery,
  triggerText,
  valueText,
  whenFrom,
  whoActors,
  type ActivityNames,
} from "./activity";

const names: ActivityNames = {
  store: (id) => ({ steam: "Steam", epic: "Epic Games" })[id] ?? id,
  plugin: (id) => ({ bsm: "BSManager" })[id] ?? id,
  status: (id) => ({ beaten: "Beaten" })[id] ?? null,
};

const cursor = { at: 0, rank: 0, id: 0 };

function entry(over: Partial<Extract<ActivityItem, { type: "entry" }>> = {}): ActivityItem {
  return {
    type: "entry",
    id: 1,
    at: "2026-11-27T14:02:00Z",
    batchId: null,
    actor: { kind: "user" },
    reason: "edit",
    args: null,
    gameId: "g1",
    gameName: "Beat Saber",
    kind: "edit",
    changeCount: 2,
    fields: ["genres", "cover"],
    cursor,
    ...over,
  };
}

const view = (change: Partial<ActivityChange>) =>
  changeView({ field: "name", mode: "value", before: null, after: null, ...change }, names);

const event = (over: Partial<SystemEvent>): SystemEvent => ({
  id: 1,
  at: "2026-11-27T11:58:00Z",
  level: "info",
  sourceKind: "app",
  sourceId: null,
  kind: "log",
  message: "raw message",
  details: null,
  cursor,
  ...over,
});

const msg = (kind: string, details: Record<string, unknown> | null = null) =>
  systemMessage(event({ kind, details }), names);

const why = (
  reason: string,
  kind: ActivityKind = "other",
  args: Record<string, unknown> | null = null,
) => ({ reason, kind, args, batchId: null }) as const;

const nested = (reason: string, kind: ActivityKind, fields: string[], args = null) =>
  nestedAction({ ...why(reason, kind, args), fields }, names);

describe("actors", () => {
  it("names each kind of actor", () => {
    expect(actorLabel({ kind: "user" }, names)).toBe("You");
    expect(actorLabel({ kind: "app" }, names)).toBe("Questkeep");
    expect(actorLabel({ kind: "store", id: "steam" }, names)).toBe("Steam");
    expect(actorLabel({ kind: "store" }, names)).toBe("Stores");
    expect(actorLabel({ kind: "plugin", id: "bsm" }, names)).toBe("BSManager plugin");
    expect(actorLabel({ kind: "plugin", id: "gone" }, names)).toBe("gone plugin");
  });

  it("keys actors like the Who filter", () => {
    expect(actorKey({ kind: "user" })).toBe("user");
    expect(actorKey({ kind: "app" })).toBe("app");
    expect(actorKey({ kind: "store", id: "steam" })).toBe("store:steam");
    expect(actorKey({ kind: "plugin", id: "bsm" })).toBe("plugin:bsm");
  });
});

describe("entry summaries", () => {
  it("phrases the common reasons", () => {
    expect(entryAction(why("metadataRefresh", "metadata"), names)).toBe("Refreshed metadata");
    expect(entryAction(why("metadataFetch"), names)).toBe("Fetched metadata");
    expect(entryAction(why("artworkFetch"), names)).toBe("Downloaded artwork");
    expect(entryAction(why("artworkRefetch"), names)).toBe("Fetched artwork again");
    expect(entryAction(why("resolveAll"), names)).toBe("Worked out the shown metadata again");
    expect(entryAction(why("combine"), names)).toBe("Combined entries");
    expect(entryAction(why("combine", "combine", { into: "Hollow Crown" }), names)).toBe(
      "Combined into “Hollow Crown”",
    );
    expect(entryAction(why("separate"), names)).toBe("Separated a store copy");
    expect(entryAction(why("preferCopy"), names)).toBe("Chose the copy to start first");
    expect(entryAction(why("bulkChange"), names)).toBe("Changed several games at once");
    expect(entryAction(why("remove"), names)).toBe("Removed from the library");
    expect(entryAction(why("remove", "removed", { excluded: true }), names)).toBe(
      "Removed from the library, not imported again",
    );
    expect(entryAction(why("importAgain", "other", { name: "Hollow Crown" }), names)).toBe(
      "Let syncs import “Hollow Crown” again",
    );
    expect(entryAction(why("addManual"), names)).toBe("Added to the library");
    expect(entryAction(why("metadataLink"), names)).toBe("Changed the metadata link");
    expect(entryAction(why("sessionEdit"), names)).toBe("Changed a play session");
    expect(entryAction(why("completionStatus"), names)).toBe("Set the completion status");
    expect(entryAction(why("completionSettings"), names)).toBe("Changed the completion statuses");
    expect(entryAction(why("setCategories"), names)).toBe("Changed the categories");
    expect(entryAction(why("autoCategories"), names)).toBe("Sorted into automatic categories");
    expect(entryAction(why("launchProfiles"), names)).toBe("Changed the launch profiles");
    expect(entryAction(why("scriptOrder"), names)).toBe("Changed the order of scripts");
    expect(entryAction(why("somethingNew"), names)).toBe("Changed");
  });

  it("says what an edit changed by its kind", () => {
    expect(entryAction(why("edit", "edit"), names)).toBe("Edited");
    expect(entryAction(why("edit", "images"), names)).toBe("Changed the artwork");
    expect(entryAction(why("edit", "status"), names)).toBe("Set the completion status");
    expect(entryAction(why("edit", "categories"), names)).toBe("Changed the categories");
    expect(entryAction(why("edit", "install"), names)).toBe("Changed the install settings");
  });

  it("phrases a game's part of a sync by its kind", () => {
    const sync = (kind: ActivityKind) => entryAction(why("sync", kind, { store: "steam" }), names);
    expect(sync("added")).toBe("Added to the library");
    expect(sync("removed")).toBe("Removed from the library");
    expect(sync("install")).toBe("Install state changed");
    expect(sync("metadata")).toBe("Updated");
    expect(entryDetail(why("sync", "added", { trigger: "manual" }), names)).toBe("during the sync");
    expect(entryDetail(why("autoSync", "added", { trigger: "startup" }), names)).toBe(
      "during the sync, automatic, at start",
    );
  });

  it("fills in the args", () => {
    expect(entryAction(why("categoryCreate", "categories", { name: "VR" }), names)).toBe(
      "Created the category “VR”",
    );
    expect(entryAction(why("categoryRename", "categories", { name: "A", to: "B" }), names)).toBe(
      "Renamed the category “A” to “B”",
    );
    expect(entryAction(why("categoryDelete", "categories", { name: "A" }), names)).toBe(
      "Deleted the category “A”",
    );
    expect(entryAction(why("categoryMerge", "categories", { name: "A", to: "B" }), names)).toBe(
      "Merged the category “A” into “B”",
    );
    expect(
      entryAction(why("categorySeparate", "categories", { name: "A", to: "Steam: A" }), names),
    ).toBe("Separated “Steam: A” from the category “A”");
    expect(
      entryAction(why("categoryFilter", "categories", { name: "A", filter: true }), names),
    ).toBe("Set the filter of the category “A”");
    expect(
      entryAction(why("categoryFilter", "categories", { name: "A", filter: false }), names),
    ).toBe("Removed the filter of the category “A”");
    expect(entryAction(why("scriptSave", "scripts", { name: "Mods" }), names)).toBe(
      "Saved the script “Mods”",
    );
    expect(entryAction(why("scriptDelete", "scripts", { name: "Mods" }), names)).toBe(
      "Deleted the script “Mods”",
    );
    expect(entryAction(why("firstStart", "status", { status: "beaten" }), names)).toBe(
      "Set to “Beaten” at the first start",
    );
    expect(entryAction(why("firstStart", "status", { status: "gone" }), names)).toBe(
      "Set to “gone” at the first start",
    );
    expect(entryAction(why("firstStart", "status"), names)).toBe(
      "Set the status at the first start",
    );
    expect(entryAction(why("restoreBackup"), names)).toBe("Restored a backup");
    expect(
      entryAction(why("restoreBackup", "other", { createdAt: "2026-11-01T10:00:00Z" }), names),
    ).toMatch(/^Restored the backup from .*2026/);
  });

  it("names the metadata sources", () => {
    expect(entryDetail(why("metadataRefresh", "metadata", { source: "igdb" }), names)).toBe(
      "from IGDB",
    );
    expect(
      entryDetail(why("metadataRefresh", "metadata", { sources: ["steam", "steamgriddb"] }), names),
    ).toBe("from Steam, SteamGridDB");
    expect(entryDetail(why("metadataRefresh"), names)).toBeNull();
    expect(entryDetail(why("edit"), names)).toBeNull();
    expect(sourceLabel("epic", names)).toBe("Epic Games");
  });

  it("writes whole sentences", () => {
    expect(itemSentence(entry({ reason: "metadataRefresh", kind: "metadata" }), names)).toBe(
      "You refreshed metadata",
    );
    expect(itemSentence(entry({ actor: { kind: "plugin", id: "bsm" } }), names)).toBe(
      "BSManager plugin edited",
    );
    expect(itemSentence(entry({ actor: { kind: "app" }, reason: "metadataFetch" }), names)).toBe(
      "Questkeep fetched metadata",
    );
    const session: ActivityItem = {
      type: "session",
      id: 4,
      gameId: "g1",
      gameName: "Hades",
      copyId: "c1",
      store: "steam",
      startedAt: "2026-11-27T08:40:00Z",
      endedAt: "2026-11-27T09:28:00Z",
      durationSeconds: 4320,
      cursor,
    };
    expect(itemSentence(session, names)).toBe("Ran for 1h 12m");
    expect(sessionText(2880)).toBe("Ran for 48m");
  });
});

describe("entries inside a batch", () => {
  it("name the changed fields instead of repeating the batch's reason", () => {
    expect(nested("bulkChange", "edit", ["favorite", "hidden"])).toBe("Favorite, Hidden");
    expect(
      nested("metadataFetch", "metadata", ["genres", "tags", "series", "links", "cover"]),
    ).toBe("Genres, Tags, Series +2");
  });

  it("keep a sync's added/updated/removed wording and fall back to the reason", () => {
    expect(nested("autoSync", "added", ["name", "installed"])).toBe("Added to the library");
    expect(nested("sync", "edit", ["installed"])).toBe(entryAction(why("sync", "edit"), names));
    expect(nested("bulkChange", "edit", [])).toBe("Changed several games at once");
  });

  it("lists fields with a +N tail", () => {
    expect(fieldsText(["hidden"])).toBe("Hidden");
    expect(fieldsText(["a", "b", "c", "d"], 2)).toBe("a, b +2");
  });
});

describe("batches", () => {
  const batch = {
    reason: "autoSync",
    args: { store: "steam", trigger: "startup" },
    counts: { added: 3, updated: 12, removed: 1 },
    gameCount: 16,
  };

  it("sums up a sync", () => {
    expect(batchAction(batch, names)).toBe("Synced");
    expect(batchDetail(batch, names)).toBe("automatic, at start");
    expect(batchDetail({ reason: "sync", args: { trigger: "manual" } }, names)).toBeNull();
    expect(batchDetail({ reason: "sync", args: { trigger: "signIn" } }, names)).toBe(
      "after signing in",
    );
    expect(batchCounts(batch)).toBe("3 added · 12 updated · 1 removed");
    expect(batchCounts({ counts: { uninstalled: 2, playtime: 1 }, gameCount: 2 })).toBe(
      "2 uninstalled · 1 running time",
    );
  });

  it("counts games for other batches", () => {
    expect(batchAction({ reason: "bulkChange", args: null }, names)).toBe(
      "Changed several games at once",
    );
    expect(batchCounts({ counts: {}, gameCount: 4 })).toBe("4 games");
    expect(batchCounts({ counts: {}, gameCount: 1 })).toBe("1 game");
    expect(batchCounts({ counts: {}, gameCount: 0 })).toBe("");
    expect(
      itemSentence(
        {
          type: "batch",
          id: 9,
          startedAt: "2026-11-27T09:15:00Z",
          finishedAt: null,
          actor: { kind: "store", id: "steam" },
          ...batch,
          entryCount: 16,
          cursor,
        },
        names,
      ),
    ).toBe("Steam synced");
  });

  it("knows every sync trigger", () => {
    expect(triggerText("addGame")).toBe("while adding a game");
    expect(triggerText("install")).toBe("after an install");
    expect(triggerText("auto")).toBe("automatic");
    expect(triggerText("installFolder")).toBe("automatic, an install folder changed");
    expect(triggerText(undefined)).toBeNull();
  });

  it("counts fields", () => {
    expect(changeCountText(3)).toBe("3 fields");
    expect(changeCountText(1)).toBe("1 field");
    expect(changeCountText(0)).toBe("");
  });
});

describe("changes", () => {
  it("labels fields", () => {
    expect(fieldLabel("genres")).toBe("Genres");
    expect(fieldLabel("copies")).toBe("Stores");
    expect(fieldLabel("userScore")).toBe("Your score");
    expect(fieldLabel("somethingNew")).toBe("somethingNew");
  });

  it("shows values before and after", () => {
    expect(view({ before: "Old", after: "New" })).toEqual({
      mode: "value",
      field: "name",
      label: "Name",
      before: "Old",
      after: "New",
    });
    expect(view({ field: "favorite", before: false, after: true })).toMatchObject({
      before: "No",
      after: "Yes",
    });
    expect(view({ field: "completionStatus", before: null, after: "beaten" })).toMatchObject({
      before: null,
      after: "Beaten",
    });
    expect(view({ field: "kind", before: "game", after: "app" })).toMatchObject({
      before: "Game",
      after: "App",
    });
  });

  it("shows sets as added and removed items", () => {
    expect(view({ field: "genres", mode: "set", before: ["Arcade"], after: ["Music"] })).toEqual({
      mode: "set",
      field: "genres",
      label: "Genres",
      removed: ["Arcade"],
      added: ["Music"],
    });
    expect(
      view({
        field: "copies",
        mode: "set",
        before: [],
        after: [{ store: "steam", storeId: "620980", qualifier: null }],
      }),
    ).toMatchObject({ added: ["Steam (620980)"], removed: [] });
    expect(
      view({
        field: "links",
        mode: "set",
        before: [{ name: "Site", url: "https://example.org" }],
        after: [{ name: "https://x.org", url: "https://x.org" }],
      }),
    ).toMatchObject({ removed: ["Site (https://example.org)"], added: ["https://x.org"] });
    expect(
      view({
        field: "launchProfiles",
        mode: "set",
        before: [],
        after: [{ name: "VR", args: "-vr" }],
      }),
    ).toMatchObject({ added: ["VR (-vr)"] });
    expect(view({ field: "tags", mode: "set", before: null, after: "odd" })).toMatchObject({
      added: [],
      removed: [],
    });
  });

  it("says images were added, removed or changed", () => {
    expect(view({ field: "cover", mode: "changed", before: false, after: true })).toMatchObject({
      label: "Cover",
      text: "added",
    });
    expect(view({ field: "icon", mode: "changed", before: true, after: false })).toMatchObject({
      text: "removed",
    });
    expect(view({ field: "background", mode: "changed", before: true, after: true })).toMatchObject(
      { text: "changed" },
    );
  });

  it("formats each kind of value", () => {
    expect(valueText("installSize", 1024 * 1024 * 512, names)).toBe("512 MB");
    expect(valueText("criticScore", { value: 85, source: "Metacritic" }, names)).toBe(
      "85 (Metacritic)",
    );
    expect(valueText("userScore", 70, names)).toBe("70");
    expect(valueText("criticScore", { source: "x" }, names)).toBeNull();
    expect(valueText("timeToBeat", { main: 3600, extra: null, complete: 7200 }, names)).toBe(
      "Main 1h · Complete 2h",
    );
    expect(valueText("timeToBeat", { main: null, extra: null, complete: null }, names)).toBeNull();
    expect(valueText("timeToBeat", "odd", names)).toBeNull();
    expect(valueText("playAction", { target: "game.exe", args: "-w" }, names)).toBe("game.exe -w");
    expect(valueText("playAction", { target: "game.exe", args: null }, names)).toBe("game.exe");
    expect(valueText("preferredCopy", { store: "epic", qualifier: "1.2" }, names)).toBe(
      "Epic Games · 1.2",
    );
    expect(valueText("metadataLink", { source: "igdb", id: "1", name: "Hades" }, names)).toBe(
      "Hades (IGDB)",
    );
    expect(valueText("metadataLink", { id: "7" }, names)).toBe("7");
    expect(valueText("metadataLink", "plain", names)).toBe("plain");
    expect(valueText("releaseDate", "2018", names)).toBe("2018");
    expect(
      valueText("session", { startedAt: "2026-11-27T08:00:00Z", durationSeconds: 600 }, names),
    ).toMatch(/· 10m$/);
    expect(valueText("session", "odd", names)).toBeNull();
    expect(valueText("defaultProfile", "VR", names)).toBe("VR");
    expect(valueText("anything", 1234, names)).toBe("1,234");
    expect(valueText("anything", { name: "Named" }, names)).toBe("Named");
    expect(valueText("anything", { other: 1 }, names)).toBe('{"other":1}');
    expect(valueText("anything", [1, 2], names)).toBe("[1,2]");
    expect(valueText("name", "", names)).toBeNull();
    expect(valueText("name", undefined, names)).toBeNull();
    expect(valueText("hidden", true, names)).toBe("Yes");
  });

  it("cuts long texts and keeps the full one for the tooltip", () => {
    const long = "word ".repeat(100);
    const shown = valueText("description", long, names) ?? "";
    expect(shown.length).toBe(MAX_VALUE_LENGTH);
    expect(shown.endsWith("…")).toBe(true);
    expect(fullValue(long)).toBe(long);
    expect(fullValue("short")).toBeNull();
    expect(fullValue(3)).toBeNull();
  });
});

describe("days and pages", () => {
  const now = new Date(2026, 10, 27, 15, 0);

  it("labels days", () => {
    expect(dayLabel(new Date(2026, 10, 27, 1).toISOString(), now)).toBe("Today");
    expect(dayLabel(new Date(2026, 10, 26, 23).toISOString(), now)).toBe("Yesterday");
    expect(dayLabel(new Date(2026, 10, 12, 18).toISOString(), now)).toBe("Nov 12, 2026");
  });

  it("groups items by local day in their order", () => {
    const items = [
      entry({ id: 1, at: new Date(2026, 10, 27, 14).toISOString() }),
      entry({ id: 2, at: new Date(2026, 10, 27, 9).toISOString() }),
      entry({ id: 3, at: new Date(2026, 10, 26, 21).toISOString() }),
      entry({ id: 4, at: new Date(2026, 10, 12, 18).toISOString() }),
    ];
    const groups = groupByDay(items, itemAt, now);
    expect(groups.map((g) => [g.label, g.items.map((i) => i.id)])).toEqual([
      ["Today", [1, 2]],
      ["Yesterday", [3]],
      ["Nov 12, 2026", [4]],
    ]);
  });

  it("finds each item's time and key", () => {
    expect(itemAt(entry())).toBe("2026-11-27T14:02:00Z");
    const batch = { type: "batch", startedAt: "b" } as unknown as ActivityItem;
    const session = { type: "session", startedAt: "s" } as unknown as ActivityItem;
    expect(itemAt(batch)).toBe("b");
    expect(itemAt(session)).toBe("s");
    expect(itemKey(entry({ id: 7 }))).toBe("entry:7");
  });

  it("appends a page without repeating items", () => {
    expect(appendPage([1, 2, 3], [3, 4], String)).toEqual([1, 2, 3, 4]);
  });
});

describe("filters", () => {
  const now = new Date("2026-11-27T15:00:00Z");

  it("maps Who to actors", () => {
    expect(whoActors("everyone")).toEqual([]);
    expect(whoActors("user")).toEqual([{ kind: "user" }]);
    expect(whoActors("app")).toEqual([{ kind: "app" }]);
    expect(whoActors("store:steam")).toEqual([{ kind: "store", id: "steam" }]);
    expect(whoActors("plugin:a:b")).toEqual([{ kind: "plugin", id: "a:b" }]);
    expect(whoActors("plugin:")).toEqual([]);
  });

  it("maps When to a start", () => {
    expect(whenFrom("all", now)).toBeUndefined();
    expect(whenFrom("7d", now)).toBe("2026-11-20T15:00:00.000Z");
    expect(whenFrom("30d", now)).toBe("2026-10-28T15:00:00.000Z");
    expect(whenFrom("90d", now)).toBe("2026-08-29T15:00:00.000Z");
    const today = new Date(whenFrom("today", now) ?? "");
    expect(today.getHours()).toBe(0);
    expect(today.getMinutes()).toBe(0);
  });

  it("builds the library query", () => {
    expect(libraryQuery(DEFAULT_LIBRARY_FILTER, now)).toEqual({
      actors: [],
      kinds: [],
      includeSessions: true,
      limit: 50,
    });
    const before = { at: 5, rank: 1, id: 2 };
    expect(
      libraryQuery(
        {
          search: " hades ",
          who: "store:steam",
          what: "metadata",
          when: "7d",
          sessions: false,
          gameId: "g1",
          batchId: null,
        },
        now,
        before,
        20,
      ),
    ).toEqual({
      actors: [{ kind: "store", id: "steam" }],
      kinds: ["metadata"],
      includeSessions: false,
      limit: 20,
      from: "2026-11-20T15:00:00.000Z",
      search: "hades",
      gameId: "g1",
      before,
    });
    // One batch's entries never mix in sessions.
    expect(libraryQuery({ ...DEFAULT_LIBRARY_FILTER, batchId: 9 }, now)).toMatchObject({
      batchId: 9,
      includeSessions: false,
    });
  });

  it("tells filtered from unfiltered", () => {
    expect(isFiltered(DEFAULT_LIBRARY_FILTER)).toBe(false);
    expect(isFiltered({ ...DEFAULT_LIBRARY_FILTER, search: "x" })).toBe(true);
    expect(isFiltered({ ...DEFAULT_LIBRARY_FILTER, gameId: "g" })).toBe(true);
    expect(isFiltered({ ...DEFAULT_LIBRARY_FILTER, sessions: false })).toBe(false);
    expect(isSystemFiltered(DEFAULT_SYSTEM_FILTER)).toBe(false);
    expect(isSystemFiltered({ ...DEFAULT_SYSTEM_FILTER, level: "error" })).toBe(true);
  });

  it("builds the system query", () => {
    expect(systemQuery(DEFAULT_SYSTEM_FILTER, now)).toEqual({ sources: [], kinds: [], limit: 50 });
    expect(
      systemQuery(
        { search: "api", source: "plugin:bsm", level: "warn", kind: "http", when: "today" },
        now,
        { at: 1, rank: 0, id: 1 },
      ),
    ).toMatchObject({
      sources: ["plugin:bsm"],
      kinds: ["http"],
      minLevel: "warn",
      search: "api",
      before: { at: 1, rank: 0, id: 1 },
    });
  });

  it("has labels for every kind", () => {
    for (const kind of SYSTEM_KINDS) expect(systemKindLabel(kind)).not.toMatch(/^sk_/);
    expect(ACTIVITY_KINDS).toContain("sessions");
  });
});

describe("system events", () => {
  it("finds the source", () => {
    expect(systemActor(event({}))).toEqual({ kind: "app" });
    expect(systemActor(event({ sourceKind: "store", sourceId: "epic" }))).toEqual({
      kind: "store",
      id: "epic",
    });
    expect(systemActor(event({ sourceKind: "plugin", sourceId: "bsm" }))).toEqual({
      kind: "plugin",
      id: "bsm",
    });
    expect(systemActor(event({ sourceKind: "plugin", sourceId: null }))).toEqual({ kind: "app" });
  });

  it("translates known kinds from their details", () => {
    expect(msg("sync", { added: 3, updated: 12 })).toBe("Synced: 3 added, 12 updated");
    expect(msg("sync", {})).toBe("Synced, no changes");
    expect(msg("backup", { size: 4 * 1024 * 1024 })).toBe("Backup written · 4 MB");
    expect(msg("backup")).toBe("Backup written");
    expect(msg("restore")).toBe("Restored a backup");
    expect(msg("updateCheck", { version: "0.9.0" })).toBe("Questkeep 0.9.0 is available");
    expect(msg("updateCheck")).toBe("Questkeep is up to date");
    expect(msg("signInExpired")).toBe("The sign-in expired");
    expect(msg("pluginUpdated", { from: "0.2.0", to: "0.3.0" })).toBe(
      "Updated from 0.2.0 to 0.3.0",
    );
    expect(msg("pluginUpdated")).toBe("raw message");
    expect(msg("pluginEnabled")).toBe("Turned on");
    expect(msg("pluginDisabled")).toBe("Turned off");
    expect(msg("pluginReloaded")).toBe("Reloaded from its source");
    expect(msg("pluginInstalled", { version: "1.0" })).toBe("Installed version 1.0");
    expect(msg("pluginInstalled")).toBe("Installed");
    expect(msg("pluginRemoved")).toBe("Removed");
    expect(msg("http", { host: "api.example.org", requests: 37, bytes: 1024 * 1024 })).toBe(
      "37 requests to api.example.org · 1 MB received",
    );
    expect(msg("http", { host: "a.org", requests: 1, bytes: 0, failed: 1 })).toBe(
      "1 request to a.org · 1 failed",
    );
    expect(msg("http", {})).toBe("raw message");
    expect(msg("notification", { title: "Epic" })).toBe("Epic: raw message");
    expect(msg("notification", { title: "raw" })).toBe("raw message");
    expect(msg("syncFailed")).toBe("raw message");
  });

  it("lists details and levels", () => {
    expect(systemDetails(event({ details: { host: "a.org", requests: 3 } }))).toEqual([
      { key: "host", value: "a.org" },
      { key: "requests", value: "3" },
    ]);
    expect(systemDetails(event({}))).toEqual([]);
    expect(levelLabel("info")).toBe("Info");
    expect(levelLabel("warn")).toBe("Warning");
    expect(levelLabel("error")).toBe("Error");
  });
});
