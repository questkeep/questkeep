import { describe, expect, it } from "vitest";
import type { Game } from "../bindings/Game";
import { hasStoreCopy } from "./removal";

const copy = (source: string, sourceGameId: string | null) => ({
  id: `${source}-${sourceGameId}`,
  source,
  sourceGameId,
  installed: false,
  playtimeSeconds: 0,
  lastPlayed: null,
});

const game = (source: string, sourceGameId: string | null, copies = [copy(source, sourceGameId)]) =>
  ({ source, sourceGameId, copies }) as unknown as Game;

describe("hasStoreCopy", () => {
  it("is true for a store game", () => {
    expect(hasStoreCopy([game("steam", "10")])).toBe(true);
  });

  it("is false for manual games only", () => {
    expect(hasStoreCopy([game("manual", null), game("manual", "x")])).toBe(false);
    expect(hasStoreCopy([])).toBe(false);
  });

  it("looks at every copy of a combined entry", () => {
    const combined = game("manual", null, [copy("manual", null), copy("gog", "20")]);
    expect(hasStoreCopy([combined])).toBe(true);
  });

  it("falls back to the game's own row without copies", () => {
    expect(hasStoreCopy([game("epic", "abc", [])])).toBe(true);
  });
});
