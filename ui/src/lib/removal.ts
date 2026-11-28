import type { Game } from "../bindings/Game";

/** The store manual games come from; they have nothing a sync could import again. */
const MANUAL_SOURCE = "manual";

/** Whether removing `games` can keep something out of later syncs (one has a store copy). */
export function hasStoreCopy(games: readonly Game[]): boolean {
  return games.some((game) =>
    (game.copies.length > 0 ? game.copies : [game]).some(
      (copy) => copy.source !== MANUAL_SOURCE && copy.sourceGameId !== null,
    ),
  );
}
