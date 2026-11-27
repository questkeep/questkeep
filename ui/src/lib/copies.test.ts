import { describe, expect, it } from "vitest";
import type { GameCopy } from "../bindings/GameCopy";
import { copyLabel } from "./copies";

const copy = (source: string, qualifier?: string): GameCopy => ({
  id: "c",
  source,
  sourceGameId: null,
  installed: true,
  playtimeSeconds: 0,
  lastPlayed: null,
  ...(qualifier === undefined ? {} : { qualifier }),
});

const names: Record<string, string> = { steam: "Steam", bsmanager: "BSManager" };
const storeName = (source: string) => names[source] ?? source;

describe("copyLabel", () => {
  it("is the store's name when the store is there once", () => {
    expect(copyLabel(copy("steam"), storeName)).toBe("Steam");
  });

  it("adds what tells copies of one store apart", () => {
    expect(copyLabel(copy("bsmanager", "1.29.1"), storeName)).toBe("BSManager · 1.29.1");
    expect(copyLabel(copy("steam", "Half-Life 2: Episode One"), storeName)).toBe(
      "Steam · Half-Life 2: Episode One",
    );
  });
});
