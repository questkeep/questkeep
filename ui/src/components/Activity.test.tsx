import { fireEvent, render, screen, waitFor, within } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ActivityItem } from "../bindings/ActivityItem";
import type { SystemEvent } from "../bindings/SystemEvent";
import * as api from "../lib/api";
import { store } from "../lib/store";
import { makeGame } from "../test-utils";
import { ActivityDrawer } from "./ActivityDrawer";
import { ActivityPage } from "./ActivityPage";
import { RecentActivityCard } from "./RecentActivityCard";

vi.mock("../lib/api");
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (p: string) => p,
  invoke: vi.fn<() => void>(),
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn<() => void>() }));

const cursor = { at: 0, rank: 0, id: 0 };
const now = new Date().toISOString();

const refresh: ActivityItem = {
  type: "entry",
  id: 1,
  at: now,
  batchId: null,
  actor: { kind: "user" },
  reason: "metadataRefresh",
  args: { source: "igdb" },
  gameId: "g1",
  gameName: "Hades",
  kind: "metadata",
  changeCount: 2,
  fields: ["genres", "cover"],
  cursor,
};
const session: ActivityItem = {
  type: "session",
  id: 2,
  gameId: "g1",
  gameName: "Hades",
  copyId: "g1",
  store: "manual",
  startedAt: now,
  endedAt: now,
  durationSeconds: 4320,
  cursor,
};
const sync: ActivityItem = {
  type: "batch",
  id: 7,
  startedAt: now,
  finishedAt: now,
  actor: { kind: "store", id: "steam" },
  reason: "autoSync",
  args: { store: "steam", trigger: "startup" },
  counts: { added: 3, updated: 12 },
  entryCount: 15,
  gameCount: 15,
  cursor,
};

beforeEach(() => {
  vi.clearAllMocks();
  store.reset();
  vi.mocked(api.activityPage).mockResolvedValue({ items: [refresh, session], next: null });
  vi.mocked(api.activityDetails).mockResolvedValue({
    entry: refresh as Extract<ActivityItem, { type: "entry" }>,
    changes: [
      { field: "genres", mode: "set", before: ["Arcade"], after: ["Music"] },
      { field: "cover", mode: "changed", before: true, after: true },
    ],
    batch: null,
  });
});

describe("Recent activity card", () => {
  it("shows the last items and opens the drawer", async () => {
    const game = makeGame();
    render(() => <RecentActivityCard game={game} />);
    expect(await screen.findByText("Refreshed metadata")).toBeInTheDocument();
    expect(screen.getByText("Ran for 1h 12m")).toBeInTheDocument();
    expect(api.activityPage).toHaveBeenCalledWith(
      expect.objectContaining({ gameId: "g1", includeSessions: true, limit: 3 }),
    );
    await fireEvent.click(screen.getByRole("button", { name: "Show all" }));
    expect(store.activityGame()?.id).toBe("g1");
  });
});

describe("Activity drawer", () => {
  it("lists a game's log, expands an entry to its changes and closes with Esc", async () => {
    const onClose = vi.fn<() => void>();
    const game = makeGame();
    render(() => <ActivityDrawer game={game} onClose={onClose} />);
    const dialog = screen.getByRole("dialog", { name: "Activity" });
    expect(document.activeElement).toBe(within(dialog).getByRole("button", { name: "Close" }));
    const row = await within(dialog).findByRole("button", {
      name: "You refreshed metadata · from IGDB · 2 fields",
    });
    expect(row).toHaveAttribute("aria-expanded", "false");
    await fireEvent.click(row);
    expect(row).toHaveAttribute("aria-expanded", "true");
    expect(api.activityDetails).toHaveBeenCalledWith(1);
    expect(await within(dialog).findByText("+ Music")).toBeInTheDocument();
    expect(within(dialog).getByText("− Arcade")).toBeInTheDocument();
    expect(within(dialog).getByText("changed")).toBeInTheDocument();
    // Down moves to the session row.
    await fireEvent.keyDown(row, { key: "ArrowDown" });
    expect(document.activeElement?.getAttribute("aria-label")).toBe(
      "Session · Ran for 1h 12m · manual",
    );
    await fireEvent.keyDown(dialog, { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
  });

  it("filters by who and opens the page filtered to the game", async () => {
    const game = makeGame();
    render(() => <ActivityDrawer game={game} onClose={vi.fn<() => void>()} />);
    await screen.findByText("Refreshed metadata");
    await fireEvent.click(screen.getByRole("combobox", { name: "Who" }));
    await fireEvent.click(screen.getByRole("option", { name: "You" }));
    await waitFor(() =>
      expect(api.activityPage).toHaveBeenLastCalledWith(
        expect.objectContaining({ gameId: "g1", actors: [{ kind: "user" }] }),
      ),
    );
    await fireEvent.click(screen.getByRole("switch", { name: "Sessions" }));
    await waitFor(() =>
      expect(api.activityPage).toHaveBeenLastCalledWith(
        expect.objectContaining({ includeSessions: false }),
      ),
    );
    await fireEvent.click(screen.getByRole("button", { name: "Open in the Activity page" }));
    expect(store.page()).toBe("activity");
    expect(store.activityRequest().library).toMatchObject({
      gameId: "g1",
      who: "user",
      sessions: false,
    });
    expect(store.activityRequest().gameName).toBe("Hades");
  });
});

describe("Activity page", () => {
  it("expands a batch to its entries and narrows to one batch", async () => {
    vi.mocked(api.activityPage).mockResolvedValue({ items: [sync], next: null });
    vi.mocked(api.activityBatchEntries).mockResolvedValue({ items: [refresh], next: null });
    store.showActivity({
      tab: "library",
      library: { batchId: 7 },
      system: {},
      batchLabel: "Steam sync",
    });
    render(() => <ActivityPage />);
    await waitFor(() =>
      expect(api.activityPage).toHaveBeenCalledWith(
        expect.objectContaining({ batchId: 7, includeSessions: false }),
      ),
    );
    const batch = await screen.findByRole("button", {
      name: "steam synced · automatic, at start · 3 added · 12 updated",
    });
    await fireEvent.click(batch);
    expect(api.activityBatchEntries).toHaveBeenCalledWith(7, null);
    expect(await screen.findByRole("button", { name: "Hades" })).toBeInTheDocument();
    // The batch says why; its entry says what changed for the game.
    expect(
      screen.getByRole("button", { name: "Hades · Genres, Cover · 2 fields" }),
    ).toBeInTheDocument();
    expect(screen.queryByText("Refreshed metadata")).not.toBeInTheDocument();
    await fireEvent.click(
      screen.getByRole("button", { name: "Remove the filter “Only Steam sync”" }),
    );
    await waitFor(() =>
      expect(api.activityPage).toHaveBeenLastCalledWith(
        expect.not.objectContaining({ batchId: expect.anything() }),
      ),
    );
  });

  it("shows library-level entries without a game", async () => {
    const created: ActivityItem = {
      ...refresh,
      id: 9,
      reason: "categoryCreate",
      args: { name: "VR" },
      gameId: null,
      gameName: "",
      kind: "other",
      changeCount: 0,
      fields: [],
    };
    vi.mocked(api.activityPage).mockResolvedValue({ items: [created], next: null });
    store.showActivity();
    render(() => <ActivityPage />);
    expect(
      await screen.findByRole("button", { name: "You created the category “VR”" }),
    ).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Hades" })).not.toBeInTheDocument();
  });

  it("shows the System part with translated messages and details", async () => {
    const event: SystemEvent = {
      id: 3,
      at: now,
      level: "warn",
      sourceKind: "plugin",
      sourceId: "meta",
      kind: "http",
      message: "37 requests",
      details: { host: "api.example.org", requests: 37, bytes: 0, failed: 2 },
      cursor,
    };
    vi.mocked(api.systemEvents).mockResolvedValue({ items: [event], next: null });
    render(() => <ActivityPage />);
    await fireEvent.click(screen.getByRole("radio", { name: "System" }));
    const row = await screen.findByRole("button", {
      name: "Warning · 37 requests to api.example.org · 2 failed",
    });
    expect(screen.getByText("meta plugin")).toBeInTheDocument();
    await fireEvent.click(row);
    expect(screen.getByText("api.example.org")).toBeInTheDocument();
    await fireEvent.click(screen.getByRole("combobox", { name: "Level" }));
    await fireEvent.click(screen.getByRole("option", { name: "Errors only" }));
    await waitFor(() =>
      expect(api.systemEvents).toHaveBeenLastCalledWith(
        expect.objectContaining({ minLevel: "error" }),
      ),
    );
  });

  it("tells an empty log from one the filters emptied", async () => {
    vi.mocked(api.activityPage).mockResolvedValue({ items: [], next: null });
    render(() => <ActivityPage />);
    expect(await screen.findByText("Nothing recorded yet.")).toBeInTheDocument();
    await fireEvent.click(screen.getByRole("combobox", { name: "What" }));
    await fireEvent.click(screen.getByRole("option", { name: "Metadata" }));
    expect(await screen.findByText("Nothing matches these filters.")).toBeInTheDocument();
  });
});

describe("sync toast details", () => {
  it("opens the page on the store's latest sync", async () => {
    vi.mocked(api.activityPage).mockResolvedValue({ items: [session, sync], next: null });
    await store.showSyncActivity(["steam"]);
    expect(api.activityPage).toHaveBeenCalledWith(
      expect.objectContaining({ actors: [{ kind: "store", id: "steam" }], includeSessions: false }),
    );
    expect(store.page()).toBe("activity");
    expect(store.activityRequest().library).toEqual({ batchId: 7 });
    expect(store.activityRequest().batchLabel).toMatch(/^steam · /);
  });

  it("filters to the store when no sync is recorded, and shows everything for several", async () => {
    vi.mocked(api.activityPage).mockResolvedValue({ items: [], next: null });
    await store.showSyncActivity(["epic"]);
    expect(store.activityRequest().library).toEqual({ who: "store:epic" });
    await store.showSyncActivity(["epic", "steam"]);
    expect(store.activityRequest().library).toEqual({});
  });
});
