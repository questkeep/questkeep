import type { Game } from "../bindings/Game";

/** The games a random pick chooses from: the view's games, only installed ones if asked. */
export function randomPool(games: readonly Game[], installedOnly: boolean): Game[] {
  return installedOnly ? games.filter((game) => game.installed) : [...games];
}

/**
 * The next random pick from `pool`, avoiding the games already shown (`seen`, oldest first) until
 * every game was shown once; then it starts over, never repeating the last one right away (unless
 * it is the only game). Returns the pick and the updated `seen`; `null` for an empty pool.
 */
export function nextPick(
  pool: readonly Game[],
  seen: readonly string[],
  random: () => number = Math.random,
): { game: Game | null; seen: string[] } {
  if (pool.length === 0) return { game: null, seen: [...seen] };
  const shown = new Set(seen);
  let fresh = pool.filter((game) => !shown.has(game.id));
  let history = [...seen];
  if (fresh.length === 0) {
    const last = seen.at(-1);
    fresh = pool.length > 1 ? pool.filter((game) => game.id !== last) : [...pool];
    history = [];
  }
  const index = Math.min(fresh.length - 1, Math.floor(random() * fresh.length));
  const game = fresh[index] ?? null;
  return { game, seen: game ? [...history, game.id] : history };
}
