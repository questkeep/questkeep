import { Match, Switch } from "solid-js";
import type { BackupImportCheck } from "../bindings/BackupImportCheck";
import { translator } from "../i18n";
import { backupImport } from "../i18n/backupImport";
import * as api from "../lib/api";
import { errorMessage } from "../lib/errors";
import { formatDateTime } from "../lib/format";
import { pickBackupFile } from "../lib/pickers";
import { store } from "../lib/store";
import { ConfirmDialog } from "./ConfirmDialog";
import { Modal } from "./Modal";

const tr = translator(backupImport);

/**
 * Lets the user pick a backup archive anywhere on disk and checks it (a foreign or too new
 * archive shows an error toast). `null` when cancelled or refused.
 */
export async function chooseBackupToImport(): Promise<BackupImportCheck | null> {
  const path = await pickBackupFile();
  if (path === null) return null;
  try {
    return await api.checkBackupImport(path);
  } catch (error) {
    store.pushToast("error", errorMessage(error));
    return null;
  }
}

/**
 * Stages the archive and restarts Questkeep. Resolves `false` (after an error toast) when it
 * failed; nothing changed then. On success the app restarts before this resolves.
 */
export async function importBackup(path: string, backUpFirst: boolean): Promise<boolean> {
  try {
    await api.importBackup(path, backUpFirst);
    return true;
  } catch (error) {
    store.pushToast("error", errorMessage(error));
    return false;
  }
}

/** The status line while an import runs, until the app restarts. */
export function importStatus(backUpFirst: boolean): string {
  return backUpFirst ? tr("backingUp") : tr("restarting");
}

/**
 * Confirms importing a checked backup. From Settings the current library is always backed up
 * first. At first run an empty library needs no backup; a library with games asks whether to
 * back it up first.
 */
export function BackupImportDialog(props: {
  check: BackupImportCheck;
  firstRun: boolean;
  onImport: (backUpFirst: boolean) => void;
  onClose: () => void;
}) {
  const params = () => ({
    date: formatDateTime(props.check.backup.createdAt),
    version: props.check.backup.appVersion ?? "?",
  });
  return (
    <Switch>
      <Match when={!props.firstRun}>
        <ConfirmDialog
          title={tr("importTitle")}
          message={tr("importMessage", params())}
          confirmLabel={tr("importConfirm")}
          onConfirm={() => props.onImport(true)}
          onClose={props.onClose}
        />
      </Match>
      <Match when={props.check.libraryEmpty}>
        <ConfirmDialog
          title={tr("restoreTitle")}
          message={tr("restoreMessage", params())}
          confirmLabel={tr("restoreConfirm")}
          onConfirm={() => props.onImport(false)}
          onClose={props.onClose}
        />
      </Match>
      <Match when={true}>
        <Modal title={tr("askTitle")} onClose={props.onClose}>
          <p class="confirm__message">{tr("askMessage", params())}</p>
          <div class="form__actions">
            <button type="button" class="btn" data-autofocus onClick={props.onClose}>
              {tr("cancel")}
            </button>
            <button
              type="button"
              class="btn btn--danger"
              onClick={() => {
                props.onImport(false);
                props.onClose();
              }}
            >
              {tr("restoreWithoutBackup")}
            </button>
            <button
              type="button"
              class="btn btn--primary"
              onClick={() => {
                props.onImport(true);
                props.onClose();
              }}
            >
              {tr("backUpAndRestore")}
            </button>
          </div>
        </Modal>
      </Match>
    </Switch>
  );
}
