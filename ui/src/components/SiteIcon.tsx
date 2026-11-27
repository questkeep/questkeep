import Globe from "lucide-solid/icons/globe";
import { createEffect, createSignal, on, Show } from "solid-js";
import { store } from "../lib/store";

/**
 * A price site's icon (its favicon, fetched and cached by the core); decorative, callers give the
 * name as text. A globe stands in while there is none: not fetched yet, offline, or not loadable.
 */
export function SiteIcon(props: { url: string }) {
  const src = () => store.siteIconUrl(props.url);
  const [broken, setBroken] = createSignal(false);
  // A new icon (refreshed, or another site) gets a new chance.
  createEffect(on(src, () => setBroken(false), { defer: true }));
  return (
    <Show when={!broken() && src()} fallback={<Globe class="icon" aria-hidden="true" />}>
      {(url) => (
        <img
          class="icon site-icon"
          src={url()}
          alt=""
          aria-hidden="true"
          draggable={false}
          onError={() => setBroken(true)}
        />
      )}
    </Show>
  );
}
