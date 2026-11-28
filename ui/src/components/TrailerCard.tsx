import ChevronLeft from "lucide-solid/icons/chevron-left";
import ChevronRight from "lucide-solid/icons/chevron-right";
import ExternalLink from "lucide-solid/icons/external-link";
import Play from "lucide-solid/icons/play";
import { createEffect, createResource, createSignal, on, Show } from "solid-js";
import type { Trailer } from "../bindings/Trailer";
import { translator } from "../i18n";
import { gameTrailers } from "../i18n/gameTrailers";
import { gameTrailers as loadTrailers } from "../lib/api";
import {
  canEmbedYouTube,
  playable,
  trailerVideo,
  youtubeEmbedUrl,
  youtubeWatchUrl,
} from "../lib/trailers";
import { Card } from "./GameSections";
import { openLink } from "./settings/openLink";
import { sourceName } from "./SourceIcon";

const tr = translator(gameTrailers);

/** From this stage width on, a trailer plays in HD unless the user chose otherwise. */
const HD_WIDTH = 860;

/**
 * The game page's trailer card: a poster that plays the trailer on click (a video file inline,
 * a YouTube video in YouTube's privacy-enhanced player where it can be embedded, else a link to
 * it). Nothing is loaded from YouTube before the click. Hidden when the game has no trailers.
 */
export function TrailerCard(props: {
  gameId: string;
  order: number;
  /** The game's background: a sharper poster than the store's small thumbnails. */
  poster?: string | null;
  /** Test seam: trailers without asking the backend. */
  load?: (id: string) => Promise<Trailer[]>;
}) {
  const [trailers] = createResource(
    () => props.gameId,
    (id) =>
      Promise.resolve()
        .then(() => (props.load ?? loadTrailers)(id))
        .then((list) => list ?? [])
        .catch(() => [] as Trailer[]),
  );
  const [index, setIndex] = createSignal(0);
  const [playing, setPlaying] = createSignal(false);
  // null: automatic (HD when the stage is wide enough), else the user's choice.
  const [hdChoice, setHdChoice] = createSignal<boolean | null>(null);
  const [wide, setWide] = createSignal(false);
  const hd = () => hdChoice() ?? wide();
  let stage: HTMLDivElement | undefined;
  const [failed, setFailed] = createSignal(false);
  const embed = canEmbedYouTube();

  createEffect(
    on(
      () => props.gameId,
      () => {
        setIndex(0);
        setPlaying(false);
        setFailed(false);
      },
      { defer: true },
    ),
  );

  const list = () => trailers() ?? [];
  const current = () => list()[Math.min(index(), list().length - 1)];

  function go(by: -1 | 1) {
    const count = list().length;
    setIndex((i) => (i + by + count) % count);
    setPlaying(false);
    setFailed(false);
  }

  function start(trailer: Trailer) {
    if (playable(trailer, embed)) {
      setWide((stage?.clientWidth ?? 0) >= HD_WIDTH);
      setFailed(false);
      setPlaying(true);
    } else if (trailer.youtubeId) {
      void openLink(youtubeWatchUrl(trailer.youtubeId));
    }
  }

  return (
    <Show when={current()}>
      {(trailer) => (
        <Card
          title={list().length > 1 ? tr("trailers") : tr("trailer")}
          order={props.order}
          class="details-card--trailer"
          action={
            <Show when={trailer().youtubeId}>
              {(id) => (
                <button
                  type="button"
                  class="link-btn"
                  onClick={() => void openLink(youtubeWatchUrl(id()))}
                >
                  {tr("watchOnYouTube")} <ExternalLink class="icon" aria-hidden="true" />
                </button>
              )}
            </Show>
          }
        >
          <div class="trailer">
            <div class="trailer__stage" ref={(el) => (stage = el)}>
              <Show
                when={playing() && !failed()}
                fallback={
                  <button
                    type="button"
                    class="trailer__poster"
                    aria-label={tr("play", { name: trailer().name || tr("trailer") })}
                    onClick={() => start(trailer())}
                  >
                    <Show when={props.poster ?? trailer().thumbnail}>
                      {(src) => <img src={src()} alt="" loading="lazy" />}
                    </Show>
                    <span class="trailer__play" aria-hidden="true">
                      <Play class="icon" />
                    </span>
                  </button>
                }
              >
                <Show
                  when={trailerVideo(trailer(), hd())}
                  fallback={
                    <iframe
                      class="trailer__frame"
                      src={youtubeEmbedUrl(trailer().youtubeId ?? "")}
                      title={trailer().name || tr("trailer")}
                      allow="autoplay; encrypted-media; picture-in-picture; fullscreen"
                      referrerPolicy="strict-origin-when-cross-origin"
                      allowfullscreen
                    />
                  }
                >
                  {(src) => (
                    // oxlint-disable-next-line jsx-a11y/media-has-caption -- store trailers come without captions
                    <video
                      class="trailer__video"
                      src={src()}
                      poster={props.poster ?? trailer().thumbnail ?? undefined}
                      controls
                      autoplay
                      preload="metadata"
                      onError={() => setFailed(true)}
                    />
                  )}
                </Show>
              </Show>
            </div>
            <Show when={failed()}>
              <p class="trailer__error" role="status">
                {tr("cantPlay")}
              </p>
            </Show>
            <div class="trailer__bar">
              <Show when={list().length > 1}>
                <button
                  type="button"
                  class="icon-btn icon-btn--small"
                  aria-label={tr("previous")}
                  data-tooltip={tr("previous")}
                  onClick={() => go(-1)}
                >
                  <ChevronLeft class="icon" aria-hidden="true" />
                </button>
                <span class="trailer__position">
                  {tr("position", { index: index() + 1, count: list().length })}
                </span>
                <button
                  type="button"
                  class="icon-btn icon-btn--small"
                  aria-label={tr("next")}
                  data-tooltip={tr("next")}
                  onClick={() => go(1)}
                >
                  <ChevronRight class="icon" aria-hidden="true" />
                </button>
              </Show>
              <span class="trailer__name">{trailer().name}</span>
              <Show when={trailer().videoHd}>
                <button
                  type="button"
                  class="btn btn--sm trailer__hd"
                  aria-pressed={hd()}
                  data-tooltip={hd() ? tr("hdOff") : tr("hdOn")}
                  onClick={() => setHdChoice(!hd())}
                >
                  {tr("hd")}
                </button>
              </Show>
              <span class="trailer__source muted">
                {tr("fromSource", {
                  source: trailer().source === "igdb" ? "IGDB" : sourceName(trailer().source),
                })}
              </span>
            </div>
          </div>
        </Card>
      )}
    </Show>
  );
}
