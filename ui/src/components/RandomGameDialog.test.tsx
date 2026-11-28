import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { listen } from "@tauri-apps/api/event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as api from "../lib/api";
import { store } from "../lib/store";
import { makeGame } from "../test-utils";
import { RandomGameDialog } from "./RandomGameDialog";

vi.mock("../lib/api");
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (p: string) => p,
  invoke: vi.fn<() => void>(),
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn<() => void>() }));

const installed = makeGame({ id: "a", name: "Abyssal Signal", installed: true });
const other = makeGame({ id: "c", name: "Cinder and Sage", installed: true });
const missing = makeGame({
  id: "b",
  name: "Bramble Bounce",
  installed: false,
  source: "steam",
  sourceGameId: "20",
});

beforeEach(async () => {
  vi.clearAllMocks();
  store.reset();
  vi.mocked(listen).mockResolvedValue(() => undefined);
  vi.mocked(api.listProviders).mockResolvedValue([]);
  vi.mocked(api.runningGames).mockResolvedValue([]);
  vi.mocked(api.listGames).mockResolvedValue([installed, missing, other]);
  vi.mocked(api.launchGame).mockResolvedValue(undefined);
  vi.mocked(api.appInfo).mockResolvedValue({
    version: "1",
    dataDir: "",
    logsDir: "",
    portable: false,
    devTools: false,
  });
  await store.init();
  await waitFor(() => {
    if (store.games().length !== 3) throw new Error("games not loaded");
  });
});

const shown = () => screen.getByRole("heading", { level: 3 }).textContent;

describe("RandomGameDialog", () => {
  it("picks an installed game and shows another one next", async () => {
    render(() => <RandomGameDialog onClose={vi.fn<() => void>()} />);
    expect(screen.getByRole("checkbox", { name: "Only installed games" })).toBeChecked();
    const first = shown();
    expect([installed.name, other.name]).toContain(first);
    await fireEvent.click(screen.getByRole("button", { name: "Pick another" }));
    expect(shown()).not.toBe(first);
    expect([installed.name, other.name]).toContain(shown());
  });

  it("plays the pick and closes", async () => {
    const onClose = vi.fn<() => void>();
    render(() => <RandomGameDialog onClose={onClose} />);
    const id = shown() === installed.name ? "a" : "c";
    await fireEvent.click(screen.getByRole("button", { name: "Play" }));
    expect(onClose).toHaveBeenCalled();
    await waitFor(() => expect(api.launchGame).toHaveBeenCalledWith(id, null, null));
  });

  it("explains an empty pick and picks from all games once unticked", async () => {
    vi.mocked(api.listGames).mockResolvedValue([missing]);
    await store.refresh();
    await waitFor(() => expect(store.games()).toHaveLength(1));
    render(() => <RandomGameDialog onClose={vi.fn<() => void>()} />);
    expect(screen.getByText(/No installed game in this view/)).toBeInTheDocument();
    await fireEvent.click(screen.getByRole("checkbox", { name: "Only installed games" }));
    expect(shown()).toBe(missing.name);
    expect(screen.getByRole("button", { name: "Install" })).toBeInTheDocument();
  });
});
