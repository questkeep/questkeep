import { fireEvent, render, screen } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import { ConfirmDialog } from "./ConfirmDialog";

function open(option?: string) {
  const onConfirm = vi.fn<(option: boolean) => void>();
  const onClose = vi.fn<() => void>();
  render(() => (
    <ConfirmDialog
      title="Remove game"
      message="Remove it?"
      confirmLabel="Remove"
      option={option}
      onConfirm={onConfirm}
      onClose={onClose}
    />
  ));
  return { onConfirm, onClose };
}

describe("ConfirmDialog option", () => {
  it("is ticked at first and confirms with it", () => {
    const { onConfirm, onClose } = open("Don't import again when syncing");
    const box = screen.getByRole("checkbox", { name: "Don't import again when syncing" });
    expect(box).toBeChecked();
    fireEvent.click(screen.getByRole("button", { name: "Remove" }));
    expect(onConfirm).toHaveBeenCalledWith(true);
    expect(onClose).toHaveBeenCalled();
  });

  it("passes false once unticked", () => {
    const { onConfirm } = open("Don't import again when syncing");
    fireEvent.click(screen.getByRole("checkbox"));
    fireEvent.click(screen.getByRole("button", { name: "Remove" }));
    expect(onConfirm).toHaveBeenCalledWith(false);
  });

  it("has no checkbox without an option and confirms with false", () => {
    const { onConfirm } = open();
    expect(screen.queryByRole("checkbox")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Remove" }));
    expect(onConfirm).toHaveBeenCalledWith(false);
  });
});
