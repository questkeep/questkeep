import { fireEvent, render, screen } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as api from "../lib/api";
import { MetadataLinkPicker } from "./MetadataLinkPicker";

vi.mock("../lib/api");

beforeEach(() => {
  vi.clearAllMocks();
});

describe("MetadataLinkPicker", () => {
  it("shows what tells same-named games apart", async () => {
    vi.mocked(api.metadataSearchSources).mockResolvedValue([{ id: "igdb", name: "IGDB" }]);
    vi.mocked(api.searchMetadata).mockResolvedValue([
      {
        id: "1",
        name: "Need for Speed: Underground 2",
        releaseYear: 2004,
        detail: "PC, Xbox · EA Black Box",
        image: null,
      },
      {
        id: "2",
        name: "Need for Speed: Underground 2",
        releaseYear: 2005,
        detail: "Game Boy Advance port · Pocket Studios",
        image: null,
      },
      { id: "3", name: "Need for Speed: Underground 2", releaseYear: null, image: null },
    ]);
    render(() => (
      <MetadataLinkPicker
        query="Need for Speed: Underground 2"
        link={null}
        onChange={vi.fn<() => void>()}
      />
    ));
    await fireEvent.click(await screen.findByRole("button", { name: "Search" }));
    expect(await screen.findByText("2004 · PC, Xbox · EA Black Box")).toBeInTheDocument();
    expect(screen.getByText("2005 · Game Boy Advance port · Pocket Studios")).toBeInTheDocument();
  });

  it("offers only IGDB for a game from a store", async () => {
    vi.mocked(api.metadataSearchSources).mockResolvedValue([
      { id: "igdb", name: "IGDB" },
      { id: "steam", name: "Steam" },
    ]);
    vi.mocked(api.searchMetadata).mockResolvedValue([]);
    const onChange = vi.fn<(link: unknown) => void>();
    render(() => <MetadataLinkPicker igdbOnly query="Doom" link={null} onChange={onChange} />);
    await fireEvent.click(await screen.findByRole("button", { name: "Search" }));
    expect(api.searchMetadata).toHaveBeenCalledWith("igdb", "Doom");
    expect(screen.queryByText("Steam")).not.toBeInTheDocument();
  });

  it("explains that IGDB needs credentials when it can't be searched", async () => {
    vi.mocked(api.metadataSearchSources).mockResolvedValue([{ id: "steam", name: "Steam" }]);
    render(() => (
      <MetadataLinkPicker igdbOnly query="Doom" link={null} onChange={vi.fn<() => void>()} />
    ));
    expect(await screen.findByText(/IGDB can't be searched yet/)).toBeInTheDocument();
  });
});
