import { describe, expect, it } from "vitest";
import {
  moveSelectIndex,
  selectKeyAction,
  typeAheadActive,
  typeAheadIndex,
  typeAheadNext,
  TYPE_AHEAD_RESET,
} from "./selectKeys";

describe("selectKeyAction", () => {
  it("opens a closed select without changing the value", () => {
    for (const key of ["Enter", " ", "ArrowDown", "ArrowUp"]) {
      expect(selectKeyAction({ key }, false)).toEqual({ kind: "open" });
    }
    expect(selectKeyAction({ key: "ArrowDown", altKey: true }, false)).toEqual({ kind: "open" });
    expect(selectKeyAction({ key: "Home" }, false)).toEqual({ kind: "openAt", at: "first" });
    expect(selectKeyAction({ key: "End" }, false)).toEqual({ kind: "openAt", at: "last" });
  });

  it("leaves other keys of a closed select alone", () => {
    expect(selectKeyAction({ key: "Escape" }, false)).toBeNull();
    expect(selectKeyAction({ key: "Tab" }, false)).toBeNull();
    expect(selectKeyAction({ key: "a", ctrlKey: true }, false)).toBeNull();
  });

  it("moves, chooses and closes an open list", () => {
    expect(selectKeyAction({ key: "ArrowDown" }, true)).toEqual({ kind: "move", to: "next" });
    expect(selectKeyAction({ key: "ArrowUp" }, true)).toEqual({ kind: "move", to: "previous" });
    expect(selectKeyAction({ key: "Home" }, true)).toEqual({ kind: "move", to: "first" });
    expect(selectKeyAction({ key: "End" }, true)).toEqual({ kind: "move", to: "last" });
    expect(selectKeyAction({ key: "PageDown" }, true)).toEqual({ kind: "move", to: "pageDown" });
    expect(selectKeyAction({ key: "Enter" }, true)).toEqual({ kind: "choose" });
    expect(selectKeyAction({ key: " " }, true)).toEqual({ kind: "choose" });
    expect(selectKeyAction({ key: "ArrowUp", altKey: true }, true)).toEqual({ kind: "choose" });
    expect(selectKeyAction({ key: "Escape" }, true)).toEqual({ kind: "close", returnFocus: true });
    expect(selectKeyAction({ key: "Tab" }, true)).toEqual({ kind: "close", returnFocus: false });
  });

  it("types printable characters, Space only while a search is going on", () => {
    expect(selectKeyAction({ key: "f" }, false)).toEqual({ kind: "type", char: "f" });
    expect(selectKeyAction({ key: "f" }, true)).toEqual({ kind: "type", char: "f" });
    expect(selectKeyAction({ key: " " }, true, true)).toEqual({ kind: "type", char: " " });
  });
});

describe("moveSelectIndex", () => {
  const none = [false, false, false, false];

  it("moves one step and stops at the ends", () => {
    expect(moveSelectIndex("next", 0, none)).toBe(1);
    expect(moveSelectIndex("next", 3, none)).toBe(3);
    expect(moveSelectIndex("previous", 2, none)).toBe(1);
    expect(moveSelectIndex("previous", 0, none)).toBe(0);
  });

  it("starts at an end when nothing is highlighted", () => {
    expect(moveSelectIndex("next", -1, none)).toBe(0);
    expect(moveSelectIndex("previous", -1, none)).toBe(3);
  });

  it("skips disabled options", () => {
    const disabled = [true, false, true, false, true];
    expect(moveSelectIndex("first", 3, disabled)).toBe(1);
    expect(moveSelectIndex("last", 1, disabled)).toBe(3);
    expect(moveSelectIndex("next", 1, disabled)).toBe(3);
    expect(moveSelectIndex("next", 3, disabled)).toBe(3);
    expect(moveSelectIndex("previous", 3, disabled)).toBe(1);
    expect(moveSelectIndex("previous", 1, disabled)).toBe(1);
  });

  it("pages by ten options", () => {
    const many = Array.from({ length: 25 }, () => false);
    expect(moveSelectIndex("pageDown", 0, many)).toBe(10);
    expect(moveSelectIndex("pageDown", 20, many)).toBe(24);
    expect(moveSelectIndex("pageUp", 15, many)).toBe(5);
    expect(moveSelectIndex("pageUp", 3, many)).toBe(0);
  });

  it("finds nothing when every option is disabled", () => {
    expect(moveSelectIndex("next", -1, [true, true])).toBe(-1);
  });
});

describe("type-ahead", () => {
  const labels = ["Deutsch", "English", "Español", "Français", "Italiano"];
  const enabled = labels.map(() => false);

  it("collects characters typed quickly and starts over after a pause", () => {
    const first = typeAheadNext(null, "E", 1000);
    expect(first).toEqual({ text: "e", at: 1000 });
    const second = typeAheadNext(first, "s", 1100);
    expect(second.text).toBe("es");
    expect(typeAheadNext(second, "f", 1100 + TYPE_AHEAD_RESET + 1).text).toBe("f");
    expect(typeAheadActive(second, 1200)).toBe(true);
    expect(typeAheadActive(second, 1100 + TYPE_AHEAD_RESET + 1)).toBe(false);
    expect(typeAheadActive(null, 0)).toBe(false);
  });

  it("finds the first label starting with the text", () => {
    expect(typeAheadIndex(labels, enabled, 0, "es")).toBe(2);
    expect(typeAheadIndex(labels, enabled, -1, "fr")).toBe(3);
    expect(typeAheadIndex(labels, enabled, 0, "xyz")).toBe(-1);
  });

  it("keeps the current option while it still matches longer text", () => {
    expect(typeAheadIndex(labels, enabled, 1, "en")).toBe(1);
  });

  it("cycles through options with the same first letter", () => {
    expect(typeAheadIndex(labels, enabled, 0, "e")).toBe(1);
    expect(typeAheadIndex(labels, enabled, 1, "e")).toBe(2);
    expect(typeAheadIndex(labels, enabled, 2, "ee")).toBe(1);
  });

  it("skips disabled options", () => {
    expect(typeAheadIndex(labels, [false, true, false, false, false], 0, "e")).toBe(2);
  });
});
