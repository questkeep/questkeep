import Undo2 from "lucide-solid/icons/undo-2";
import { createResource, For, Show } from "solid-js";
import type { ImportExclusion } from "../../bindings/ImportExclusion";
import { translator } from "../../i18n";
import { settingsExclusions } from "../../i18n/settingsExclusions";
import * as api from "../../lib/api";
import { errorMessage } from "../../lib/errors";
import { formatDate } from "../../lib/format";
import { store } from "../../lib/store";
import { SourceIcon, sourceName } from "../SourceIcon";

const tr = translator(settingsExclusions);

/** The store games syncs skip because the user removed them; each can be imported again. */
export function ExcludedGames() {
  const [list, { refetch }] = createResource(api.listImportExclusions);

  async function importAgain(entry: ImportExclusion) {
    try {
      await api.removeImportExclusion(entry.source, entry.sourceGameId);
      await refetch();
      // A store that is gone (an uninstalled plugin) imports it once it is back.
      if (store.providers().some((p) => p.id === entry.source)) {
        store.pushToast(
          "info",
          tr("importing", { name: entry.name, store: sourceName(entry.source) }),
        );
        // Progress and failures arrive as scan events, like any sync.
        void api
          .scanLibrary(entry.source)
          .then(() => store.scheduleRefresh())
          .catch(() => {});
      }
    } catch (error) {
      store.pushToast("error", errorMessage(error));
    }
  }

  return (
    <section class="settings-section exclusions" aria-labelledby="excluded-games-heading">
      <h3 id="excluded-games-heading" class="settings-part__title">
        {tr("title")}
      </h3>
      <div class="settings-card exclusions__card">
        <p class="muted">{tr("intro")}</p>
        <Show when={(list() ?? []).length > 0} fallback={<p class="muted">{tr("empty")}</p>}>
          <ul class="exclusions__list" aria-label={tr("list")}>
            <For each={list()}>
              {(entry) => (
                <li class="exclusions__item">
                  <SourceIcon source={entry.source} />
                  <span class="exclusions__text">
                    <span class="exclusions__name">{entry.name}</span>
                    <small class="muted">
                      {tr("excludedOn", {
                        store: sourceName(entry.source),
                        date: formatDate(entry.excludedAt),
                      })}
                    </small>
                  </span>
                  <button
                    type="button"
                    class="btn"
                    aria-label={tr("importAgainLabel", { name: entry.name })}
                    onClick={() => void importAgain(entry)}
                  >
                    <Undo2 class="icon" aria-hidden="true" />
                    {tr("importAgain")}
                  </button>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </div>
    </section>
  );
}
