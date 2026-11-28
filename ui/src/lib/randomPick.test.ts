import { describe, expect, it } from "vitest";
import { makeGame } from "../test-utils";
import { nextPick, randomPool } from "./randomPick";

const games = [
  makeGame({ id: "a", name: "Alpha", installed: true }),
  makeGame({ id: "b", name: "Beta", installed: false }),
  makeGame({ id: "c", name: "Gamma", installed: true }),
];

/** A "random" source that returns these values in turn. */
const sequence = (...values: number[]) => {
  let i = 0;
  return () => values[i++ % values.length] ?? 0;
};

describe("random pick", () => {
  it("keeps only installed games when asked", () => {
    expect(randomPool(games, true).map((g) => g.id)).toEqual(["a", "c"]);
    expect(randomPool(games, false)).toHaveLength(3);
  });

  it("picks by the random value and remembers what it showed", () => {
    expect(nextPick(games, [], () => 0).game?.id).toBe("a");
    expect(nextPick(games, [], () => 0.99).game?.id).toBe("c");
    expect(nextPick(games, [], () => 0.5)).toEqual({ game: games[1], seen: ["b"] });
  });

  it("shows every game once before it repeats one", () => {
    let seen: string[] = [];
    const picked: string[] = [];
    const random = sequence(0, 0.99, 0.5);
    for (let i = 0; i < 3; i++) {
      const next = nextPick(games, seen, random);
      seen = next.seen;
      picked.push(next.game?.id ?? "");
    }
    expect(picked.toSorted()).toEqual(["a", "b", "c"]);
  });

  it("starts over without repeating the last pick right away", () => {
    const next = nextPick(games, ["a", "b", "c"], () => 0.99);
    expect(next.game?.id).toBe("b");
    expect(next.seen).toEqual(["b"]);
    expect(nextPick(games, ["c", "b", "a"], () => 0).game?.id).toBe("b");
  });

  it("repeats the only game, and picks nothing from an empty pool", () => {
    const one = [games[0] as (typeof games)[number]];
    expect(nextPick(one, ["a"]).game?.id).toBe("a");
    expect(nextPick([], ["a"])).toEqual({ game: null, seen: ["a"] });
  });

  it("stays in range when random returns 1", () => {
    expect(nextPick(games, [], () => 1).game?.id).toBe("c");
  });
});
