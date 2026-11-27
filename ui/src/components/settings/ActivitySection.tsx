import { createResource, createSignal, For, Show } from "solid-js";
import type { ActivityPart } from "../../bindings/ActivityPart";
import type { ActivitySettings } from "../../bindings/ActivitySettings";
import { translator } from "../../i18n";
import { activity } from "../../i18n/activity";
import * as api from "../../lib/api";
import { errorMessage } from "../../lib/errors";
import { formatDateTime, formatSize } from "../../lib/format";
import { store } from "../../lib/store";
import { ConfirmDialog } from "../ConfirmDialog";
import { Select } from "../Select";
import type { SettingsModel } from "./useSettings";

const tr = translator(activity);

const SYSTEM_DAYS = [7, 14, 30, 90, 180, 365];
const MIN_MB = 5;
const MAX_MB = 2048;
const PARTS: { value: ActivityPart; label: "s_clearLibrary" | "s_clearSystem" | "s_clearAll" }[] = [
  { value: "library", label: "s_clearLibrary" },
  { value: "system", label: "s_clearSystem" },
  { value: "all", label: "s_clearAll" },
];

/** The share of the limit each part takes, for the bar. */
const share = (bytes: number, limitMb: number) =>
  `${Math.min(100, (bytes / (limitMb * 1024 * 1024)) * 100).toFixed(1)}%`;

/** Settings › Activity: how much history is kept, its size, and clearing it. */
export function ActivitySection(props: { model: SettingsModel }) {
  const [version, setVersion] = createSignal(0);
  const [stats] = createResource(version, () => api.activityStats().catch(() => null));
  const [clearing, setClearing] = createSignal(false);
  const [part, setPart] = createSignal<ActivityPart>("library");

  function change(patch: Partial<ActivitySettings>) {
    const current = props.model.settings()?.activity;
    if (!current) return;
    void props.model.update({ activity: { ...current, ...patch } });
  }

  async function clear(which: ActivityPart) {
    try {
      await api.clearActivity(which);
      store.pushToast("info", tr("s_cleared"));
    } catch (error) {
      store.pushToast("error", errorMessage(error));
    }
    setVersion((n) => n + 1);
  }

  return (
    <Show when={props.model.settings()?.activity} fallback={<p class="muted">{tr("loading")}</p>}>
      {(current) => (
        <div class="form">
          <fieldset class="fieldset settings-group">
            <legend class="fieldset__legend">{tr("s_storage")}</legend>
            <Show when={stats()} fallback={<p class="muted">{tr("loading")}</p>}>
              {(s) => (
                <div class="alog-storage">
                  <div class="alog-storage__bar" aria-hidden="true">
                    <i
                      class="alog-storage__library"
                      style={{ width: share(s().libraryBytes, current().libraryMaxMb) }}
                    />
                    <i
                      class="alog-storage__system"
                      style={{ width: share(s().systemBytes, current().libraryMaxMb) }}
                    />
                  </div>
                  <p class="alog-storage__legend">
                    <span class="alog-storage__key alog-storage__key--library">
                      {tr("s_library", { size: formatSize(s().libraryBytes) })}
                    </span>
                    <span class="alog-storage__key alog-storage__key--system">
                      {tr("s_system", { size: formatSize(s().systemBytes) })}
                    </span>
                    <span>
                      {tr("s_ofLimit", { limit: formatSize(current().libraryMaxMb * 1024 * 1024) })}
                    </span>
                  </p>
                  <p class="field__hint">
                    {tr("s_counts", {
                      entries: s().entries.toLocaleString(),
                      events: s().systemEvents.toLocaleString(),
                    })}
                    {" · "}
                    {tr("s_file", { size: formatSize(s().fileBytes) })}
                    <Show when={s().oldestEntry}>
                      {(oldest) => <> · {tr("s_oldest", { date: formatDateTime(oldest()) })}</>}
                    </Show>
                  </p>
                </div>
              )}
            </Show>
            <div class="settings-row">
              <label for="activity-library-mb">{tr("s_libraryLimit")}</label>
              <span class="alog-number">
                <input
                  id="activity-library-mb"
                  class="input settings-row__number"
                  type="number"
                  min={MIN_MB}
                  max={MAX_MB}
                  value={current().libraryMaxMb}
                  aria-describedby="activity-library-mb-hint"
                  onChange={(e) => {
                    const mb = Math.round(Number(e.currentTarget.value));
                    if (Number.isFinite(mb) && mb >= MIN_MB && mb <= MAX_MB) {
                      change({ libraryMaxMb: mb });
                    } else e.currentTarget.value = String(current().libraryMaxMb);
                  }}
                />
                {tr("s_mb")}
              </span>
            </div>
            <p class="field__hint" id="activity-library-mb-hint">
              {tr("s_libraryLimitHint")}
            </p>
            <div class="settings-row">
              <label for="activity-system-days">{tr("s_systemDays")}</label>
              <Select
                id="activity-system-days"
                class="input settings-row__input"
                value={current().systemDays}
                options={[...new Set([...SYSTEM_DAYS, current().systemDays])]
                  .toSorted((a, b) => a - b)
                  .map((days) => ({ value: days, label: tr("s_days", { count: days }) }))}
                aria-describedby="activity-system-days-hint"
                onChange={(systemDays) => change({ systemDays })}
              />
            </div>
            <p class="field__hint" id="activity-system-days-hint">
              {tr("s_systemDaysHint")}
            </p>
            <label class="check check--switch">
              <input
                type="checkbox"
                role="switch"
                checked={current().recordPluginRequests}
                aria-describedby="activity-requests-hint"
                onChange={(e) => change({ recordPluginRequests: e.currentTarget.checked })}
              />
              <span>{tr("s_recordRequests")}</span>
            </label>
            <p class="field__hint" id="activity-requests-hint">
              {tr("s_recordRequestsHint")}
            </p>
          </fieldset>
          <div class="settings-card">
            <div class="scan-row">
              <button
                type="button"
                class="btn btn--danger"
                onClick={() => {
                  setPart("library");
                  setClearing(true);
                }}
              >
                {`${tr("s_clear")}…`}
              </button>
              <span class="muted">{tr("s_clearHint")}</span>
            </div>
          </div>
          <Show when={clearing()}>
            <ConfirmDialog
              title={tr("s_clear")}
              message={
                <>
                  {tr("s_clearMessage")}
                  <span class="alog-parts" role="radiogroup" aria-label={tr("s_clear")}>
                    <For each={PARTS}>
                      {(option) => (
                        <label class="check">
                          <input
                            type="radio"
                            name="activity-clear-part"
                            value={option.value}
                            checked={part() === option.value}
                            onChange={() => setPart(option.value)}
                          />
                          <span>{tr(option.label)}</span>
                        </label>
                      )}
                    </For>
                  </span>
                </>
              }
              confirmLabel={tr("s_clearConfirm")}
              onConfirm={() => void clear(part())}
              onClose={() => setClearing(false)}
            />
          </Show>
        </div>
      )}
    </Show>
  );
}
