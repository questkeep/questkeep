import { convertFileSrc } from "@tauri-apps/api/core";
import { createResource, type Accessor } from "solid-js";
import type { Game } from "../bindings/Game";
import { toolIcons } from "./api";
import { store } from "./store";

/**
 * The pictures of a game's tools, by tool id: the picked icon, else the program's icon or the
 * link's favicon. Asked again when the game changes (saving tools reports that) and when a
 * favicon arrives.
 */
export function createToolIcons(game: Accessor<Game>): Accessor<Map<string, string>> {
  const [icons] = createResource(
    () => ({ id: game().id, changed: game(), version: store.siteIconsVersion() }),
    ({ id, version }) =>
      Promise.resolve()
        .then(() => toolIcons(id))
        .then(
          (files) =>
            new Map(
              (files ?? []).map((file) => [
                file.toolId,
                `${convertFileSrc(file.path)}?v=${version}`,
              ]),
            ),
        )
        .catch(() => new Map<string, string>()),
  );
  return () => icons.latest ?? new Map<string, string>();
}
