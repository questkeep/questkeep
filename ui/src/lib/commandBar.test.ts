import { describe, expect, it } from "vitest";
import { makeGame } from "../test-utils";
import type { MenuEntry } from "../components/Menu";
import {
  COMMAND_LIMIT,
  COMMAND_RESULTS,
  flattenMenu,
  matchCommands,
  rankMatches,
  splitMatch,
  type AppCommand,
} from "./commandBar";

const game = (id: string, name: string, hidden = false) =>
  makeGame({ id, name, sortingName: name.toLowerCase(), hidden });

const ids = (commands: readonly AppCommand[]) => commands.map((c) => c.id);

const command = (id: string, label: string, keywords?: string): AppCommand => ({
  id,
  label,
  keywords,
  run: () => {},
});

describe("command bar", () => {
  it("offers the commands whose label matches the search", () => {
    const commands = [command("notifications", "Show notifications")];
    expect(matchCommands(commands, "")).toEqual([]);
    expect(matchCommands(commands, "  NOTIF ")).toEqual(commands);
    expect(matchCommands(commands, "witcher")).toEqual([]);
  });

  it("needs every word of the search, in the label or the keywords", () => {
    const commands = [
      command("steam", "Sync › Steam"),
      command("epic", "Sync › Epic Games"),
      command("home", "Go to Home", "start page"),
    ];
    expect(ids(matchCommands(commands, "sync steam"))).toEqual(["steam"]);
    expect(ids(matchCommands(commands, "start"))).toEqual(["home"]);
    expect(ids(matchCommands(commands, "sync xbox"))).toEqual([]);
  });

  it("ranks label starts, then word starts, then other places, then keywords", () => {
    const commands = [
      command("keyword", "Go to Home", "statistics overview"),
      command("inside", "Show lostats"),
      command("word", "Go to Statistics"),
      command("start", "Start page settings"),
    ];
    expect(ids(matchCommands(commands, "sta", 4))).toEqual(["start", "word", "inside", "keyword"]);
  });

  it("prefers shorter labels and keeps at most the limit", () => {
    const commands = [
      command("long", "Sync › Steam (all libraries)"),
      command("short", "Sync › Steam"),
      command("all", "Sync all"),
      command("epic", "Sync › Epic Games"),
    ];
    expect(ids(matchCommands(commands, "sync"))).toEqual(["all", "short", "epic"]);
    expect(matchCommands(commands, "sync")).toHaveLength(COMMAND_LIMIT);
  });

  it("turns a menu into commands with the submenu path in the label", () => {
    const ran: string[] = [];
    const entries: MenuEntry[] = [
      { label: "Edit views", onSelect: () => ran.push("edit") },
      { kind: "separator" },
      {
        kind: "submenu",
        label: "Sync",
        entries: () => [
          { label: "Sync all", onSelect: () => ran.push("all") },
          { label: "Sync later", disabled: true, onSelect: () => ran.push("later") },
          { kind: "heading", label: "Stores" },
          { label: "Steam", hint: "Ctrl+1", onSelect: () => ran.push("steam") },
        ],
      },
    ];
    const commands = flattenMenu(entries);
    expect(commands.map((c) => [c.label, c.hint])).toEqual([
      ["Edit views", undefined],
      ["Sync all", undefined],
      ["Sync › Steam", "Ctrl+1"],
    ]);
    expect(new Set(ids(commands)).size).toBe(3);
    for (const c of commands) c.run();
    expect(ran).toEqual(["edit", "all", "steam"]);
  });

  it("ranks visible games first, then earlier matches, then by name", () => {
    const ranked = rankMatches(
      [
        game("1", "The Witcher"),
        game("2", "Witcher 2"),
        game("3", "Witch Hunt", true),
        game("4", "Awitch"),
      ],
      "witch",
    );
    expect(ranked.map((g) => g.id)).toEqual(["2", "4", "1", "3"]);
  });

  it("keeps a handful of results", () => {
    const many = Array.from({ length: 20 }, (_, i) => makeGame({ id: `${i}`, name: `Game ${i}` }));
    expect(rankMatches(many, "game")).toHaveLength(COMMAND_RESULTS);
  });

  it("splits a name around the match, ignoring case", () => {
    expect(splitMatch("Hollow Knight", "kNI")).toEqual({
      before: "Hollow ",
      match: "Kni",
      after: "ght",
    });
    expect(splitMatch("Hades", "zz")).toEqual({ before: "Hades", match: "", after: "" });
    expect(splitMatch("Hades", " ")).toEqual({ before: "Hades", match: "", after: "" });
  });
});
