import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import type { GameTool } from "../bindings/GameTool";
import * as api from "../lib/api";
import { makeGame } from "../test-utils";
import { EditToolsList } from "./EditToolsList";

vi.mock("../lib/api");
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (p: string) => p,
  invoke: vi.fn<() => void>(),
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn<() => void>() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn<() => void>() }));

const wiki: GameTool = {
  id: "t1",
  name: "Wiki",
  icon: null,
  action: {
    kind: "uri",
    target: "https://example.com/wiki",
    args: null,
    workingDir: null,
    tracking: "default",
    trackingPath: null,
  },
};

async function openMenu() {
  const menuButton = screen
    .getAllByRole("button")
    .find((b) => b.getAttribute("aria-haspopup") === "menu");
  if (!menuButton) throw new Error("no menu button");
  await fireEvent.click(menuButton);
}

describe("the tools list's menu", () => {
  it("edits, duplicates and deletes a tool", async () => {
    const onChange = vi.fn<(tools: GameTool[]) => void>();
    render(() => <EditToolsList game={makeGame()} tools={[wiki]} onChange={onChange} />);

    await openMenu();
    await fireEvent.click(screen.getByRole("menuitem", { name: "Edit" }));
    expect(screen.getByDisplayValue("https://example.com/wiki")).toBeInTheDocument();

    await openMenu();
    await fireEvent.click(screen.getByRole("menuitem", { name: "Duplicate" }));
    const duplicated = onChange.mock.lastCall?.[0];
    expect(duplicated?.map((t) => t.name)).toEqual(["Wiki", "Wiki (copy)"]);
    expect(duplicated?.[1]?.id).not.toBe("t1");

    await openMenu();
    await fireEvent.click(screen.getByRole("menuitem", { name: "Delete" }));
    expect(onChange).toHaveBeenLastCalledWith([]);
  });

  it("previews the automatic icon (the favicon) of a tool without its own", async () => {
    vi.mocked(api.toolIcons).mockResolvedValue([{ toolId: "t1", path: "C:/data/fav.png" }]);
    vi.mocked(api.toolDefaultIcon).mockResolvedValue("C:/data/fav.png");
    const { container } = render(() => (
      <EditToolsList game={makeGame()} tools={[wiki]} onChange={vi.fn<() => void>()} />
    ));
    await openMenu();
    await fireEvent.click(screen.getByRole("menuitem", { name: "Edit" }));
    await waitFor(() =>
      expect(
        container.querySelector(".tools-edit__icon-preview img")?.getAttribute("src"),
      ).toContain("fav.png"),
    );
  });
});
