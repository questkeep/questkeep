import { type JSX, Show, createSignal, untrack } from "solid-js";
import { translator } from "../i18n";
import { shellCommon } from "../i18n/shellCommon";
import { Modal } from "./Modal";

const tr = translator(shellCommon);

interface ConfirmDialogProps {
  title: string;
  message: JSX.Element;
  confirmLabel: string;
  /** A checkbox under the message (ticked at first); its state goes to `onConfirm`. */
  option?: string;
  onConfirm: (option: boolean) => void;
  onClose: () => void;
}

/**
 * Asks before a destructive action; Cancel has the focus, so Enter does not confirm by accident.
 * Confirming runs `onConfirm` before `onClose`: closing may unmount the owner (a context menu),
 * after which its props can no longer be read.
 */
export function ConfirmDialog(props: ConfirmDialogProps) {
  const [checked, setChecked] = createSignal(true);
  // Read once: it cannot change while the dialog is open, and a tracked read would follow a
  // closing owner's stale props (a context menu that unmounts on confirm).
  const option = untrack(() => props.option);
  return (
    <Modal title={props.title} onClose={props.onClose}>
      <p class="confirm__message">{props.message}</p>
      <Show when={option}>
        {(label) => (
          <label class="check confirm__option">
            <input
              type="checkbox"
              checked={checked()}
              onChange={(event) => setChecked(event.currentTarget.checked)}
            />
            {label()}
          </label>
        )}
      </Show>
      <div class="form__actions">
        <button type="button" class="btn" data-autofocus onClick={props.onClose}>
          {tr("cancel")}
        </button>
        <button
          type="button"
          class="btn btn--danger-solid"
          onClick={() => {
            props.onConfirm(option !== undefined && checked());
            props.onClose();
          }}
        >
          {props.confirmLabel}
        </button>
      </div>
    </Modal>
  );
}
