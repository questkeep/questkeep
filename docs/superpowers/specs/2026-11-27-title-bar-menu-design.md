# Add game menu: sections, quick access, "Activate on"

Date: 2026-11-27. Status: accepted.

## Problem

The title bar's "Add game" (+) menu lists every way to add a game, then "Buy from <store>" for
every store and "Compare on <site>" for every price site, flat. With a dozen stores it is long, and
there is no way to redeem a game key. A key dialog that detects the store from the key's format was
considered and dropped: key formats collide (Microsoft, EA and Meta all use 5×5), several stores
have no known format, and only Steam and GOG can take a key in a link, so the result would behave
differently per store.

## Outcome

```
+ menu                                   Settings → Window and tray → Title bar
──────────────────────────────           ┌ Quick access   [ 2 items ▾ ] ┐
Add a game…            (quick access)    │ IN THE MENU (drag to reorder) │
Activate on Steam      (quick access)    │ ⠿ Add a game…              ✕  │
──────────────────────────────           │ ⠿ Activate on Steam        ✕  │
Add               ▸  Manual game…        │ ───────────────────────────── │
                     New Minecraft inst. │ ▾ Add                         │
Compare on        ▸  GG.deals            │     ☑ Manual game…            │
Buy from          ▸  Steam, Epic, …      │     ☐ New Minecraft instance  │
Activate on       ▸  Steam, Epic, …      │ ▸ Compare on                  │
                                         │ ▸ Buy from                    │
                                         │ ▾ Activate on                 │
                                         │     ☑ Steam                   │
                                         └───────────────────────────────┘
```

- The menu starts with the user's **quick access** items, then a separator, then the submenus
  **Add**, **Compare on**, **Buy from**, **Activate on**. Empty submenus are left out. Items in
  quick access stay in their submenu too.
- A quick-access item is labelled with its submenu's name in front: "Buy from Steam", "Activate on
  Steam", "Compare on GG.deals", "Add New Minecraft instance". The one exception is the manual add,
  labelled "Add a game…" (in the Add submenu: "Manual game…").
- Default quick access: the manual add only, so the most common action is still one click away
  after the update.
- "Activate on <store>" opens the store's page or dialog for redeeming a product key. Questkeep
  never sees or handles keys.
- The app menu's "Add game" submenu shows the same entries.
- Extension (JavaScript) entries for the `add` menu location stay at the end of the Add submenu;
  they can't be put in quick access (they have no stable id). Later, if asked for.

## Where the entries come from

Stores declare all of them with `add_game_options`; only "Compare on" stays in the app (price
sites are an app setting, Settings → Price sites).

`AddGameOption` gets a `section` (`add` default, `buy`, `activate`; serde default keeps existing
plugins working). Builders: `AddGameOption::buy(id, label)` and `::activate(id, label)`, combined
with the existing `opening_browser` / `opening_launcher[_with_keys]`. For `buy` and `activate` the
label is the store's name (it follows "Buy from" / "Activate on").

`LibraryProvider::shop_url` and `ProviderInfo::shop_url` are removed; the stores' websites become
`buy` options. The plugin SDK's `Provider::shop_url` stays one release as a fallback: a plugin that
sets it and declares no `buy` option gets a `buy` option opening that URL in the browser (the host
does this, so old plugins keep their "Buy from" entry). Marked deprecated.

`AddGameTarget` gets a `Uri { uri }` variant: a store URI the system opens (`steam://…`,
`com.epicgames.launcher://…`). Built-in stores only: the plugin host drops options with it (a
plugin can't open arbitrary URIs; it uses `Launcher { args }`).

Options can now depend on the store's settings, so the trait method takes the store's config:
`fn add_game_options(&self, config: &serde_json::Value)`. The WASM host replays `configure(config)`
when it differs from the config it last applied (plugins read their settings in `configure`, as
before; the SDK signature is unchanged).

Which options the app shows: `add` options and anything opening the launcher (`Launcher`,
`Provider`, `Uri`) only while the store is installed, as today; `buy`/`activate` options opening a
web page (`Browser`) always.

Only `add` options start the "watch for the new game" worker after they run; buying or activating
doesn't install anything.

## Launcher or browser: a store option

Stores whose launcher can open the same page offer a store option (Settings → Stores, the store's
row) **"Open store pages in the browser"**, unticked by default (store options are checkboxes that
are unset by default, so the key says "browser"): by default "Buy from" and "Activate on" open in
the launcher; ticked, both open in the browser. A store whose launcher isn't installed returns the
browser targets regardless. Stores without launcher pages have no option and always use the browser.
This is entirely the store's business: core only sees the targets.

## Per store

| Store | Buy from | Activate on | Option |
|---|---|---|---|
| Steam | launcher `steam://openurl/https://store.steampowered.com/`, browser `https://store.steampowered.com/` | launcher `steam://open/activateproduct`, browser `https://store.steampowered.com/account/registerkey` | yes |
| Epic Games | launcher `com.epicgames.launcher://store`, browser `https://store.epicgames.com/` | browser `https://store.epicgames.com/redeem` (no launcher link known) | yes (buy only) |
| GOG | browser `https://www.gog.com/` | browser `https://www.gog.com/redeem` | no |
| Ubisoft Connect | browser `https://store.ubisoft.com/` | launcher (open Ubisoft Connect: "Activate a key" is in its menu; there is no web page) | no |
| EA app | browser `https://www.ea.com/games` | browser `https://www.ea.com/redeem` | no |
| Xbox | browser `https://www.xbox.com/` | browser `https://redeem.microsoft.com/` | no |
| Battle.net | browser `https://shop.battle.net/` | browser `https://account.battle.net/` (Account overview has "Redeem a code") | no |
| Meta | browser `https://www.meta.com/experiences/` | browser `https://secure.oculus.com/redeem-code` | no |
| Amazon | browser `https://gaming.amazon.com/` | none (Prime Gaming codes are redeemed at the store the offer names) | no |
| Heroic, Lutris, manual | none | none | no |

Launcher links beyond Steam's and Epic's store page could not be confirmed (research 2026-11-27:
no store fills a key into its launcher through a link); add them per store when found.

## Settings

`Settings.title_bar: TitleBarSettings { quick_access: Vec<String> }`, default `["add:manual"]`.
Item ids:

- `add:manual`
- `add:<provider>/<option>`, `buy:<provider>/<option>`, `activate:<provider>/<option>`
- `compare:<price site url>`

Ids whose store, plugin or site is gone are kept in the list but not shown (they return with it).
Order is the user's.

## The settings control

Settings → Window and tray gets a **Title bar** section with one row, "Quick access", and a
dropdown button showing the count ("2 items", "No items"). Its popup:

- Top: "In the menu", the chosen items in order, each with a grip handle (drag to reorder;
  Alt+Up/Down on the focused row), the label exactly as the menu shows it, and a remove button.
  With nothing chosen, a hint: "Check items below to add them to the menu."
- Below: a tree of the four sections (only non-empty ones), each item with a checkbox. Checking
  appends to the end of the list, unchecking removes it. Keyboard: arrows move, Right/Left expand
  and collapse a section, Space toggles.
- Unavailable saved items (store gone) are not shown here either.

Pure logic in `ui/src/lib/addMenu.ts` (vitest): building the sections from providers and price
sites, item ids, quick-access labels, resolving the saved ids, toggling and moving ids.

## Not changing

- How add options run (launcher keys, browser https check, the watch worker for `add`).
- The tray menu, the command bar.
