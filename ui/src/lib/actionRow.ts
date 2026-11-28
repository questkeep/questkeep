/**
 * How many lines a wrapping flex row needs for items of these natural widths, laid out greedily
 * like `flex-wrap: wrap`. An item as wide as the row (or wider) fills a line by itself.
 */
export function lineCount(widths: readonly number[], row: number, gap: number): number {
  let lines = 0;
  let used = 0;
  for (const width of widths) {
    if (lines > 0 && used + gap + width <= row + 0.5) {
      used += gap + width;
    } else {
      lines += 1;
      used = width;
    }
  }
  return lines;
}

/** The widths of a game's tool and script buttons with names, as icons, and of their "…". */
export interface ActionWidths {
  labels: number[];
  icons: number[];
  more: number;
}

/** What the game page's action row measured. */
export interface ActionRowSizes {
  /** Play (at its smallest) with its store arrow. */
  play: number;
  actions: ActionWidths;
  /** The gap between Play and the action buttons, and between those. */
  launchGap: number;
  /** The stats block with all header buttons, and with only "…". */
  metaFull: number;
  metaFolded: number;
  /** The row's width and the gap between Play's group and the stats block. */
  row: number;
  gap: number;
  /** Play's group has a line of its own (narrow page). */
  narrow: boolean;
}

/** How the row is drawn. */
export interface ActionRowPlan {
  /** Action buttons with their names; else as icons. */
  labels: boolean;
  /** How many action buttons are shown; the rest are under their "…". */
  shown: number;
  /** Favorite, Hide and Edit are under the header's "…". */
  foldMeta: boolean;
}

/** Play's group with the first `shown` action buttons, named or not, and "…" for the rest. */
function launchWidth(sizes: ActionRowSizes, labels: boolean, shown: number): number {
  const widths = (labels ? sizes.actions.labels : sizes.actions.icons).slice(0, shown);
  const all = sizes.actions.labels.length;
  const parts = [sizes.play, ...widths, ...(shown < all ? [sizes.actions.more] : [])];
  return parts.reduce((sum, w) => sum + w, 0) + sizes.launchGap * (parts.length - 1);
}

/**
 * Plans the game page's action row. Best first: everything on one line with named buttons, then
 * with icons, then with the header buttons folded. Otherwise Play's group takes a line of its
 * own (always so on a narrow page) with named buttons, icons, or as many icons as fit and "…",
 * and the stats block goes below, folded when it does not fit on its line.
 */
export function planActionRow(sizes: ActionRowSizes): ActionRowPlan {
  const all = sizes.actions.labels.length;
  const fits = (width: number) => width <= sizes.row + 0.5;
  if (!sizes.narrow) {
    const oneLine: ActionRowPlan[] = [
      { labels: true, shown: all, foldMeta: false },
      { labels: false, shown: all, foldMeta: false },
      { labels: false, shown: all, foldMeta: true },
    ];
    for (const plan of oneLine) {
      const meta = plan.foldMeta ? sizes.metaFolded : sizes.metaFull;
      if (fits(launchWidth(sizes, plan.labels, plan.shown) + sizes.gap + meta)) return plan;
    }
  }
  const foldMeta = !fits(sizes.metaFull);
  if (fits(launchWidth(sizes, true, all))) return { labels: true, shown: all, foldMeta };
  for (let shown = all; shown > 0; shown--) {
    if (fits(launchWidth(sizes, false, shown))) return { labels: false, shown, foldMeta };
  }
  return { labels: false, shown: 0, foldMeta };
}
