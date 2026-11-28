import { describe, expect, it } from "vitest";

import { type ActionRowSizes, lineCount, planActionRow } from "./actionRow";

describe("lineCount", () => {
  it("lays items out greedily", () => {
    expect(lineCount([], 500, 16)).toBe(0);
    expect(lineCount([180, 400], 600, 16)).toBe(1);
    expect(lineCount([180, 400], 590, 16)).toBe(2);
    // Wider than the row: a line of its own.
    expect(lineCount([100, 700, 100], 500, 16)).toBe(3);
  });
});

/** Play 180, two tools (100 named, 50 as icons), "…" 36, stats 300 + 16 + buttons. */
function sizes(row: number, narrow = false): ActionRowSizes {
  return {
    play: 180,
    actions: { labels: [100, 100], icons: [50, 50], more: 36 },
    launchGap: 8,
    metaFull: 300 + 16 + 160,
    metaFolded: 300 + 16 + 36,
    row,
    gap: 20,
    narrow,
  };
}

describe("planActionRow", () => {
  it("keeps everything on one line with names when it fits", () => {
    // Launch 180 + 8 + 100 + 8 + 100 = 396; + 20 + 476 = 892.
    expect(planActionRow(sizes(900))).toEqual({ labels: true, shown: 2, foldMeta: false });
  });

  it("drops the names, then folds the header buttons, to stay on one line", () => {
    // Icons: 180 + 8 + 50 + 8 + 50 = 296; + 20 + 476 = 792.
    expect(planActionRow(sizes(800))).toEqual({ labels: false, shown: 2, foldMeta: false });
    // Folded: 296 + 20 + 352 = 668.
    expect(planActionRow(sizes(680))).toEqual({ labels: false, shown: 2, foldMeta: true });
  });

  it("gives Play's group a line of its own when one line is not enough", () => {
    expect(planActionRow(sizes(500))).toEqual({ labels: true, shown: 2, foldMeta: false });
    expect(planActionRow(sizes(450))).toEqual({ labels: true, shown: 2, foldMeta: true });
  });

  it("shows icons, then as many as fit with a … on a narrow page", () => {
    expect(planActionRow(sizes(400, true))).toEqual({ labels: true, shown: 2, foldMeta: true });
    expect(planActionRow(sizes(300, true))).toEqual({ labels: false, shown: 2, foldMeta: true });
    // One icon and "…": 180 + 8 + 50 + 8 + 36 = 282.
    expect(planActionRow(sizes(290, true))).toEqual({ labels: false, shown: 1, foldMeta: true });
    expect(planActionRow(sizes(200, true))).toEqual({ labels: false, shown: 0, foldMeta: true });
  });

  it("is just Play without actions", () => {
    const none = { ...sizes(700), actions: { labels: [], icons: [], more: 36 } };
    expect(planActionRow(none)).toEqual({ labels: true, shown: 0, foldMeta: false });
  });
});
