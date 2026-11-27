import { fireEvent, render, screen, within } from "@solidjs/testing-library";
import { createSignal } from "solid-js";
import { describe, expect, it, vi } from "vitest";
import { chooseOption } from "../test-utils";
import { Select, type SelectItem } from "./Select";

const LANGUAGES: SelectItem<string>[] = [
  { value: "", label: "System language" },
  { value: "de", label: "Deutsch" },
  { value: "en", label: "English" },
  { value: "es", label: "Español", disabled: true },
  { value: "fr", label: "Français" },
];

function Harness(props: {
  options?: SelectItem<string>[];
  initial?: string;
  onChange?: (value: string) => void;
  disabled?: boolean;
}) {
  const [value, setValue] = createSignal(props.initial ?? "en");
  return (
    <>
      <label for="language">Language</label>
      <Select
        id="language"
        class="input"
        value={value()}
        options={props.options ?? LANGUAGES}
        disabled={props.disabled}
        onChange={(next) => {
          setValue(next);
          props.onChange?.(next);
        }}
      />
      <output data-testid="value">{value()}</output>
    </>
  );
}

const trigger = () => screen.getByRole("combobox", { name: "Language" });
const shown = () => screen.getByTestId("value").textContent;

describe("Select", () => {
  it("is a labelled button showing the chosen option", () => {
    render(() => <Harness />);
    const button = trigger();
    expect(button.tagName).toBe("BUTTON");
    expect(screen.getByLabelText("Language")).toBe(button);
    expect(button).toHaveTextContent("English");
    expect(button).toHaveAttribute("aria-haspopup", "listbox");
    expect(button).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("opens a listbox on click, marks the chosen option and picks another", async () => {
    const onChange = vi.fn<(value: string) => void>();
    render(() => <Harness onChange={onChange} />);
    await fireEvent.click(trigger());
    expect(trigger()).toHaveAttribute("aria-expanded", "true");
    const list = screen.getByRole("listbox", { name: "Language" });
    expect(trigger()).toHaveAttribute("aria-controls", list.id);
    expect(within(list).getByRole("option", { name: "English" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(within(list).getByRole("option", { name: "Deutsch" })).toHaveAttribute(
      "aria-selected",
      "false",
    );
    expect(within(list).getByRole("option", { name: "Español" })).toHaveAttribute(
      "aria-disabled",
      "true",
    );
    await fireEvent.click(within(list).getByRole("option", { name: "Français" }));
    expect(onChange).toHaveBeenCalledWith("fr");
    expect(shown()).toBe("fr");
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(trigger()).toHaveTextContent("Français");
    expect(trigger()).toHaveFocus();
  });

  it("ignores clicks on disabled options", async () => {
    const onChange = vi.fn<(value: string) => void>();
    render(() => <Harness onChange={onChange} />);
    await fireEvent.click(trigger());
    await fireEvent.click(screen.getByRole("option", { name: "Español" }));
    expect(onChange).not.toHaveBeenCalled();
    expect(screen.getByRole("listbox")).toBeInTheDocument();
  });

  it("works with the chooseOption test helper", async () => {
    render(() => <Harness />);
    await chooseOption(trigger(), "Deutsch");
    expect(shown()).toBe("de");
  });

  it("opens with the keyboard on the chosen option, moves past disabled ones and chooses", async () => {
    const onChange = vi.fn<(value: string) => void>();
    render(() => <Harness onChange={onChange} />);
    const button = trigger();
    button.focus();
    await fireEvent.keyDown(button, { key: "ArrowDown" });
    expect(screen.getByRole("listbox")).toBeInTheDocument();
    // Closed, arrows only open: the value is unchanged.
    expect(onChange).not.toHaveBeenCalled();
    const active = () =>
      document.getElementById(button.getAttribute("aria-activedescendant") ?? "");
    expect(active()).toHaveTextContent("English");
    await fireEvent.keyDown(button, { key: "ArrowDown" });
    expect(active()).toHaveTextContent("Français"); // Español is disabled
    await fireEvent.keyDown(button, { key: "ArrowDown" });
    expect(active()).toHaveTextContent("Français"); // stops at the end
    await fireEvent.keyDown(button, { key: "Home" });
    expect(active()).toHaveTextContent("System language");
    await fireEvent.keyDown(button, { key: "End" });
    await fireEvent.keyDown(button, { key: "ArrowUp" });
    expect(active()).toHaveTextContent("English");
    await fireEvent.keyDown(button, { key: "Enter" });
    expect(onChange).not.toHaveBeenCalled(); // English was already chosen
    expect(screen.queryByRole("listbox")).toBeNull();
    await fireEvent.keyDown(button, { key: "ArrowUp" });
    await fireEvent.keyDown(button, { key: "ArrowUp" });
    await fireEvent.keyDown(button, { key: " " });
    expect(onChange).toHaveBeenCalledWith("de");
  });

  it.each([{ key: "Enter" }, { key: " " }, { key: "ArrowDown", altKey: true }])(
    "opens with %o",
    async (init) => {
      render(() => <Harness />);
      await fireEvent.keyDown(trigger(), init);
      expect(screen.getByRole("listbox")).toBeInTheDocument();
    },
  );

  it("closes on Escape without changing the value or reaching a dialog around it", async () => {
    const outer = vi.fn<(event: KeyboardEvent) => void>();
    const escapes = () => outer.mock.calls.filter(([event]) => event.key === "Escape").length;
    render(() => (
      <div onKeyDown={(event) => outer(event)}>
        <Harness />
      </div>
    ));
    trigger().focus();
    await fireEvent.keyDown(trigger(), { key: "Enter" });
    await fireEvent.keyDown(trigger(), { key: "ArrowUp" });
    await fireEvent.keyDown(trigger(), { key: "Escape" });
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(shown()).toBe("en");
    expect(trigger()).toHaveFocus();
    expect(escapes()).toBe(0);
    // Closed, Escape is left to the dialog.
    await fireEvent.keyDown(trigger(), { key: "Escape" });
    expect(escapes()).toBe(1);
  });

  it("closes on Tab", async () => {
    render(() => <Harness />);
    await fireEvent.keyDown(trigger(), { key: "Enter" });
    await fireEvent.keyDown(trigger(), { key: "Tab" });
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(shown()).toBe("en");
  });

  it("finds options by typing their first letters", async () => {
    render(() => <Harness />);
    const button = trigger();
    await fireEvent.keyDown(button, { key: "f" });
    const active = () =>
      document.getElementById(button.getAttribute("aria-activedescendant") ?? "");
    expect(screen.getByRole("listbox")).toBeInTheDocument();
    expect(active()).toHaveTextContent("Français");
    await fireEvent.keyDown(button, { key: "Enter" });
    expect(shown()).toBe("fr");
  });

  it("closes when clicking outside, toggles on the button", async () => {
    render(() => <Harness />);
    await fireEvent.click(trigger());
    await fireEvent.pointerDown(document.body);
    expect(screen.queryByRole("listbox")).toBeNull();
    await fireEvent.click(trigger());
    expect(screen.getByRole("listbox")).toBeInTheDocument();
    await fireEvent.click(trigger());
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("does not open while disabled", async () => {
    render(() => <Harness disabled />);
    expect(trigger()).toBeDisabled();
    await fireEvent.keyDown(trigger(), { key: "Enter" });
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("shows option groups under their heading", async () => {
    render(() => (
      <Harness
        initial="last"
        options={[
          { value: "last", label: "Last page" },
          {
            group: "Views",
            options: [
              { value: "view:a", label: "All games" },
              { value: "view:b", label: "Favorites" },
            ],
          },
        ]}
      />
    ));
    await fireEvent.click(trigger());
    const group = screen.getByRole("group", { name: "Views" });
    expect(
      within(group)
        .getAllByRole("option")
        .map((o) => o.textContent),
    ).toEqual(["All games", "Favorites"]);
    await fireEvent.keyDown(trigger(), { key: "ArrowDown" });
    await fireEvent.keyDown(trigger(), { key: "Enter" });
    expect(shown()).toBe("view:a");
  });

  it("keeps number values as numbers", async () => {
    const onChange = vi.fn<(value: number) => void>();
    render(() => (
      <Select<number>
        aria-label="Every"
        value={15}
        options={[
          { value: 0, label: "Off" },
          { value: 15, label: "15 minutes" },
        ]}
        onChange={onChange}
      />
    ));
    await chooseOption(screen.getByRole("combobox", { name: "Every" }), "Off");
    expect(onChange).toHaveBeenCalledWith(0);
  });
});
