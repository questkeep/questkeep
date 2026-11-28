import { describe, expect, it } from "vitest";
import type { Trailer } from "../bindings/Trailer";
import { canEmbedYouTube, playable, trailerVideo, youtubeEmbedUrl } from "./trailers";

const video: Trailer = {
  name: "Launch",
  thumbnail: null,
  video: "https://cdn/x/movie480.mp4",
  videoHd: "https://cdn/x/movie_max.mp4",
  youtubeId: null,
  source: "steam",
};
const youtube: Trailer = { ...video, video: null, videoHd: null, youtubeId: "abc", source: "igdb" };

describe("trailers", () => {
  it("embeds YouTube only from an http(s) origin", () => {
    expect(canEmbedYouTube("http://tauri.localhost")).toBe(true);
    expect(canEmbedYouTube("tauri://localhost")).toBe(false);
  });

  it("picks the HD file only when asked and present", () => {
    expect(trailerVideo(video, false)).toBe("https://cdn/x/movie480.mp4");
    expect(trailerVideo(video, true)).toBe("https://cdn/x/movie_max.mp4");
    expect(trailerVideo({ ...video, videoHd: null }, true)).toBe("https://cdn/x/movie480.mp4");
    expect(trailerVideo(youtube, true)).toBeNull();
  });

  it("knows what plays on the page", () => {
    expect(playable(video, false)).toBe(true);
    expect(playable(youtube, true)).toBe(true);
    expect(playable(youtube, false)).toBe(false);
    expect(youtubeEmbedUrl("a b")).toBe(
      "https://www.youtube-nocookie.com/embed/a%20b?autoplay=1&rel=0",
    );
  });
});
