import type { Trailer } from "../bindings/Trailer";

/**
 * Whether YouTube's player can be embedded here. YouTube refuses to play in pages without an
 * http(s) origin ("Error 153"): the Linux and macOS webviews serve the app from `tauri://`, the
 * Windows one from `http://tauri.localhost`, where embedding works.
 */
export function canEmbedYouTube(origin: string = globalThis.location?.origin ?? ""): boolean {
  return origin.startsWith("http://") || origin.startsWith("https://");
}

/** The privacy-enhanced embed of a YouTube video, started right away (it loads on a click). */
export const youtubeEmbedUrl = (id: string) =>
  `https://www.youtube-nocookie.com/embed/${encodeURIComponent(id)}?autoplay=1&rel=0`;

export const youtubeWatchUrl = (id: string) =>
  `https://www.youtube.com/watch?v=${encodeURIComponent(id)}`;

/** The video file to play: the HD one when asked for and there is one. */
export function trailerVideo(trailer: Trailer, hd: boolean): string | null {
  return (hd ? (trailer.videoHd ?? trailer.video) : (trailer.video ?? trailer.videoHd)) ?? null;
}

/** Whether a trailer can be played on the page at all (else the card links out). */
export function playable(trailer: Trailer, embedYouTube: boolean): boolean {
  return trailerVideo(trailer, false) !== null || (trailer.youtubeId !== null && embedYouTube);
}
