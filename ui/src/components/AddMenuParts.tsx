import BadgePercent from "lucide-solid/icons/badge-percent";
import KeyRound from "lucide-solid/icons/key-round";
import Plus from "lucide-solid/icons/plus";
import ShoppingCart from "lucide-solid/icons/shopping-cart";
import type { JSX } from "solid-js";
import { translator } from "../i18n";
import { shellTitleBar } from "../i18n/shellTitleBar";
import { shopping } from "../i18n/shopping";
import {
  addMenuGroups,
  quickLabel,
  type AddMenuGroup,
  type AddMenuItem,
  type AddMenuSection,
} from "../lib/addMenu";
import { store } from "../lib/store";
import { SiteIcon } from "./SiteIcon";
import { SourceIcon } from "./SourceIcon";

const tr = translator(shellTitleBar);
const trShop = translator(shopping);

/** The "Add game" menu's entries as they are now, by submenu. */
export const currentAddMenuGroups = (): AddMenuGroup[] =>
  addMenuGroups(store.providers(), store.priceSites(), tr("manualGame"));

/** A submenu's name: "Add", "Compare on", "Buy from", "Activate on". */
export function sectionName(section: AddMenuSection): string {
  switch (section) {
    case "add":
      return tr("menuAdd");
    case "compare":
      return tr("menuCompare");
    case "buy":
      return tr("menuBuy");
    case "activate":
      return tr("menuActivate");
  }
}

/** An entry's label in quick access: "Buy from Steam", "Add a game…". */
export const quickItemLabel = (item: AddMenuItem): string =>
  quickLabel(
    item,
    (section, name) => {
      switch (section) {
        case "add":
          return tr("quickAdd", { name });
        case "compare":
          return trShop("compareOn", { site: name });
        case "buy":
          return trShop("buyFrom", { store: name });
        case "activate":
          return tr("activateOn", { store: name });
      }
    },
    tr("addGameEllipsis"),
  );

/** An entry's icon: its store's logo, the price site's icon, or + for the manual add. */
export function AddMenuIcon(props: { item: AddMenuItem }): JSX.Element {
  const action = () => props.item.action;
  return (
    <>
      {(() => {
        const a = action();
        if (a.kind === "option") return <SourceIcon source={a.providerId} />;
        if (a.kind === "site") return <SiteIcon url={a.url} />;
        return <Plus class="icon" aria-hidden="true" />;
      })()}
    </>
  );
}

/** A submenu's icon. */
export function SectionIcon(props: { section: AddMenuSection }): JSX.Element {
  return (
    <>
      {(() => {
        switch (props.section) {
          case "add":
            return <Plus class="icon" aria-hidden="true" />;
          case "compare":
            return <BadgePercent class="icon" aria-hidden="true" />;
          case "buy":
            return <ShoppingCart class="icon" aria-hidden="true" />;
          case "activate":
            return <KeyRound class="icon" aria-hidden="true" />;
        }
      })()}
    </>
  );
}
