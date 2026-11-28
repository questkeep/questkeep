import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import type { Trailer } from "../bindings/Trailer";
import { TrailerCard } from "./TrailerCard";

vi.mock("../lib/api");
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (p: string) => p,
  invoke: vi.fn<() => void>(),
}));
vi.mock("./settings/openLink", () => ({ openLink: vi.fn<() => Promise<void>>() }));

const steam: Trailer = {
  name: "Launch Trailer",
  thumbnail: "https://cdn/thumb.jpg",
  video: "https://cdn/1/movie480.mp4",
  videoHd: "https://cdn/1/movie_max.mp4",
  youtubeId: null,
  source: "steam",
};
const second: Trailer = { ...steam, name: "Gameplay", video: "https://cdn/2/movie480.mp4" };
const youtube: Trailer = {
  name: "Reveal",
  thumbnail: null,
  video: null,
  videoHd: null,
  youtubeId: "abc",
  source: "igdb",
};

describe("TrailerCard", () => {
  it("shows nothing without trailers", async () => {
    const load = vi.fn<(id: string) => Promise<Trailer[]>>().mockResolvedValue([]);
    const { container } = render(() => <TrailerCard gameId="g1" order={2} load={load} />);
    await waitFor(() => expect(load).toHaveBeenCalledWith("g1"));
    expect(container.querySelector(".details-card")).toBeNull();
  });

  it("plays a video only after a click, in HD when asked, and steps through trailers", async () => {
    const load = vi.fn<() => Promise<Trailer[]>>().mockResolvedValue([steam, second]);
    const { container } = render(() => <TrailerCard gameId="g1" order={2} load={load} />);
    const play = await screen.findByRole("button", { name: "Play Launch Trailer" });
    expect(container.querySelector("video")).toBeNull();
    expect(screen.getByText("1 of 2")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "HD" }));
    fireEvent.click(play);
    expect(container.querySelector("video")?.getAttribute("src")).toBe(
      "https://cdn/1/movie_max.mp4",
    );

    fireEvent.click(screen.getByRole("button", { name: "Next trailer" }));
    expect(container.querySelector("video")).toBeNull();
    expect(screen.getByRole("button", { name: "Play Gameplay" })).toBeTruthy();
    expect(screen.getByText("2 of 2")).toBeTruthy();
  });

  it("embeds YouTube from an http origin, with nothing loaded before the click", async () => {
    const load = vi.fn<() => Promise<Trailer[]>>().mockResolvedValue([youtube]);
    const { container } = render(() => <TrailerCard gameId="g1" order={2} load={load} />);
    const play = await screen.findByRole("button", { name: "Play Reveal" });
    expect(container.querySelector("iframe")).toBeNull();
    fireEvent.click(play);
    // jsdom's origin is http://localhost:3000, like the Windows webview's http origin.
    expect(container.querySelector("iframe")?.getAttribute("src")).toBe(
      "https://www.youtube-nocookie.com/embed/abc?autoplay=1&rel=0",
    );
    expect(screen.getByRole("button", { name: /Watch on YouTube/ })).toBeTruthy();
  });
});
