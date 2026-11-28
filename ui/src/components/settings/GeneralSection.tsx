import { For, Show } from "solid-js";
import type { AutoShortcuts } from "../../bindings/AutoShortcuts";
import type { StartupPage } from "../../bindings/StartupPage";
import { LOCALE_NAMES, LOCALES, resolveLocale, translator } from "../../i18n";
import { settingsGeneral } from "../../i18n/settingsGeneral";
import { allViews, shownName } from "../../lib/views";
import { store } from "../../lib/store";
import { ChipInput } from "../ChipInput";
import { Select } from "../Select";
import { CheckOption } from "./SettingsControls";
import type { SettingsModel } from "./useSettings";

/** Common leading words offered while typing (English, German, French, Spanish). */
const SUGGESTED_PREFIXES = ["The", "A", "An", "Der", "Die", "Das", "Le", "La", "Les", "El", "Los"];

const tr = translator(settingsGeneral);

/** The automatic sync's choices, in minutes (0: off). */
const AUTO_SYNC_MINUTES = [0, 15, 60] as const;

/** The language the system would give (shown next to "System language"). */
const systemLocale = () => resolveLocale(null, navigator.languages ?? [navigator.language]);

/** The dropdown value of a startup page: "start", "last" or "view:<id>". */
function startupValue(page: StartupPage): string {
  return page.kind === "view" ? `view:${page.id}` : page.kind;
}

function startupPage(value: string): StartupPage {
  if (value.startsWith("view:")) return { kind: "view", id: value.slice("view:".length) };
  return value === "last" ? { kind: "last" } : { kind: "start" };
}

export function GeneralSection(props: { model: SettingsModel }) {
  return (
    <Show when={props.model.settings()} fallback={<p class="muted">{tr("loading")}</p>}>
      {(current) => (
        <>
          <fieldset class="fieldset settings-group">
            <legend class="fieldset__legend">{tr("language")}</legend>
            <div class="settings-row">
              <label for="language">{tr("language")}</label>
              <Select
                id="language"
                class="input settings-row__input"
                value={current().language ?? ""}
                options={[
                  {
                    value: "",
                    label: tr("systemLanguage", { name: LOCALE_NAMES[systemLocale()] }),
                  },
                  ...LOCALES.map((locale) => ({ value: locale, label: LOCALE_NAMES[locale] })),
                ]}
                onChange={(language) => void props.model.update({ language: language || null })}
              />
            </div>
          </fieldset>
          <fieldset class="fieldset settings-group">
            <legend class="fieldset__legend">{tr("syncGroup")}</legend>
            <CheckOption
              model={props.model}
              option="scanOnStartup"
              checked={current().scanOnStartup}
            />
            <div class="settings-row">
              <label for="auto-sync">{tr("autoSync")}</label>
              <Select<number>
                id="auto-sync"
                class="input settings-row__input"
                aria-describedby="auto-sync-hint"
                value={current().autoSyncMinutes}
                options={AUTO_SYNC_MINUTES.map((minutes) => ({
                  value: minutes,
                  label:
                    minutes === 0
                      ? tr("autoSyncOff")
                      : minutes === 60
                        ? tr("autoSyncHourly")
                        : tr("autoSyncMinutes", { minutes }),
                }))}
                onChange={(minutes) => void props.model.update({ autoSyncMinutes: minutes })}
              />
            </div>
            <p class="field__hint" id="auto-sync-hint">
              {tr("autoSyncHint")}
            </p>
            <CheckOption
              model={props.model}
              option="watchInstallFolders"
              checked={current().watchInstallFolders}
            />
            <div class="settings-row">
              <label for="auto-shortcuts">{tr("autoShortcuts")}</label>
              <Select<AutoShortcuts>
                id="auto-shortcuts"
                class="input settings-row__input"
                aria-describedby="auto-shortcuts-hint"
                value={current().autoShortcuts}
                options={[
                  { value: "off", label: tr("autoShortcutsOff") },
                  { value: "desktop", label: tr("autoShortcutsDesktop") },
                  { value: "startMenu", label: tr("autoShortcutsStartMenu") },
                  { value: "both", label: tr("autoShortcutsBoth") },
                ]}
                onChange={(autoShortcuts) => void props.model.update({ autoShortcuts })}
              />
            </div>
            <p class="field__hint" id="auto-shortcuts-hint">
              {tr("autoShortcutsHint")}
            </p>
          </fieldset>
          <fieldset class="fieldset settings-group">
            <legend class="fieldset__legend">{tr("playtimeGroup")}</legend>
            <CheckOption
              model={props.model}
              option="detectExternalLaunches"
              checked={current().detectExternalLaunches}
            />
          </fieldset>
          <fieldset class="fieldset settings-group">
            <legend class="fieldset__legend">{tr("pagesGroup")}</legend>
            <label class="check check--switch">
              <input
                type="checkbox"
                role="switch"
                checked={current().showStartPage}
                onChange={(e) =>
                  void props.model.update({ showStartPage: e.currentTarget.checked })
                }
              />
              <span>{tr("showStartPage")}</span>
            </label>
            <Show when={current().showStartPage}>
              <For
                each={
                  [
                    ["background", "startBackground"],
                    ["recentlyPlayed", "startRecentlyPlayed"],
                    ["recentlyAdded", "startRecentlyAdded"],
                    ["statistics", "startStats"],
                  ] as const
                }
              >
                {([field, label]) => (
                  <label class="check check--switch check--sub">
                    <input
                      type="checkbox"
                      role="switch"
                      checked={current().startPage[field]}
                      onChange={(e) =>
                        void props.model.update({
                          startPage: { ...current().startPage, [field]: e.currentTarget.checked },
                        })
                      }
                    />
                    <span>{tr(label)}</span>
                  </label>
                )}
              </For>
            </Show>
            <label class="check check--switch">
              <input
                type="checkbox"
                role="switch"
                checked={current().showStatistics}
                onChange={(e) =>
                  void props.model.update({ showStatistics: e.currentTarget.checked })
                }
              />
              <span>{tr("showStatistics")}</span>
            </label>
            <div class="settings-row">
              <label for="startup-page">{tr("whenOpens")}</label>
              <Select
                id="startup-page"
                class="input settings-row__input"
                value={startupValue(current().startupPage)}
                options={[
                  { value: "start", label: tr("startPage"), disabled: !current().showStartPage },
                  { value: "last", label: tr("lastPage") },
                  {
                    group: tr("views"),
                    options: allViews(store.sections()).map((view) => ({
                      value: `view:${view.id}`,
                      label: shownName(view),
                    })),
                  },
                ]}
                onChange={(value) => void props.model.update({ startupPage: startupPage(value) })}
              />
            </div>
            <Show when={!current().showStartPage && current().startupPage.kind === "start"}>
              <p class="field__hint">{tr("startPageOff")}</p>
            </Show>
          </fieldset>
          <fieldset class="fieldset settings-group">
            <legend class="fieldset__legend">{tr("sorting")}</legend>
            <label class="check check--switch">
              <input
                type="checkbox"
                role="switch"
                checked={current().sortPrefixes.enabled}
                onChange={(e) =>
                  void props.model.update({
                    sortPrefixes: { ...current().sortPrefixes, enabled: e.currentTarget.checked },
                  })
                }
              />
              <span>{tr("ignorePrefixes")}</span>
            </label>
            <div
              class="settings-row settings-row--stacked"
              classList={{ "check--disabled": !current().sortPrefixes.enabled }}
            >
              <label for="sort-prefixes">{tr("wordsToIgnore")}</label>
              <ChipInput
                id="sort-prefixes"
                label={tr("wordsToIgnore")}
                values={current().sortPrefixes.words}
                suggestions={SUGGESTED_PREFIXES}
                describedBy="sort-prefixes-hint"
                onChange={(words) =>
                  void props.model.update({ sortPrefixes: { ...current().sortPrefixes, words } })
                }
              />
            </div>
            <p class="field__hint" id="sort-prefixes-hint">
              {tr("prefixesHint")}
            </p>
          </fieldset>
          <fieldset class="fieldset settings-group">
            <legend class="fieldset__legend">{tr("notificationsGroup")}</legend>
            <label class="check check--switch">
              <input
                type="checkbox"
                role="switch"
                checked={current().systemNotifications}
                aria-describedby="system-notifications-hint"
                onChange={(e) =>
                  void props.model.toggle("systemNotifications", e.currentTarget.checked)
                }
              />
              <span>{tr("systemNotifications")}</span>
            </label>
            <p class="field__hint" id="system-notifications-hint">
              {tr("systemNotificationsHint")}
            </p>
          </fieldset>
        </>
      )}
    </Show>
  );
}
