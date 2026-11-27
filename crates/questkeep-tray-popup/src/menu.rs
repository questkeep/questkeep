//! The game menu (right click, Shift+click, the context-menu key or Shift+F10 on a game): what it
//! lists, pure so it is tested here; `window` shows it as a native menu.
//!
//! ```text
//! Open game page          Middle click
//! ─────────────────────────────────────
//! Play with Steam              Default    one entry per launch profile, in the user's order
//! Play with Epic Games
//! ```

use uuid::Uuid;

use super::layout::{Action, PopupTexts};

/// A launch profile of a game, as the app hands it over when the menu opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopupProfile {
    pub id: Uuid,
    pub name: String,
    /// The profile Play (a left click) starts.
    pub default: bool,
    /// It can be played now (its store copy is installed).
    pub available: bool,
}

/// An entry of a native menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuEntry {
    Item {
        text: String,
        /// Shown right-aligned in the shortcut column.
        hint: Option<String>,
        action: Action,
        enabled: bool,
        /// Drawn bold: what a left click on the game does.
        default: bool,
    },
    Separator,
}

/// The menu of `game` with its launch `profiles` (in the user's order). Without profiles (none
/// could be read) it offers plain Play instead, so the menu always has a way to play.
pub fn game_menu(game: Uuid, profiles: &[PopupProfile], texts: &PopupTexts) -> Vec<MenuEntry> {
    let mut entries = vec![
        MenuEntry::Item {
            text: texts.open_game.clone(),
            hint: Some(texts.middle_click.clone()),
            action: Action::OpenGame(game),
            enabled: true,
            default: false,
        },
        MenuEntry::Separator,
    ];
    if profiles.is_empty() {
        entries.push(MenuEntry::Item {
            text: texts.play.clone(),
            hint: None,
            action: Action::Launch(game),
            enabled: true,
            default: true,
        });
    }
    entries.extend(profiles.iter().map(|p| MenuEntry::Item {
        text: texts.play_with_label(&p.name),
        hint: p.default.then(|| texts.default_profile.clone()),
        action: Action::LaunchProfile {
            game,
            profile: p.id,
        },
        enabled: p.available,
        default: p.default,
    }));
    entries
}

/// Menu text for a Win32 menu: `&` is literal (not an access key) and the hint goes after a tab
/// into the right-aligned shortcut column.
pub fn native_text(text: &str, hint: Option<&str>) -> String {
    let text = text.replace('&', "&&");
    match hint {
        Some(hint) => format!("{text}\t{}", hint.replace('&', "&&")),
        None => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(n: u128, name: &str, default: bool, available: bool) -> PopupProfile {
        PopupProfile {
            id: Uuid::from_u128(n),
            name: name.into(),
            default,
            available,
        }
    }

    fn item(entry: &MenuEntry) -> (&str, Option<&str>, &Action, bool, bool) {
        match entry {
            MenuEntry::Item {
                text,
                hint,
                action,
                enabled,
                default,
            } => (text, hint.as_deref(), action, *enabled, *default),
            MenuEntry::Separator => panic!("a separator"),
        }
    }

    #[test]
    fn game_page_first_then_one_entry_per_profile() {
        let game = Uuid::from_u128(1);
        let texts = PopupTexts::default();
        let profiles = [
            profile(10, "Steam", false, true),
            profile(11, "Epic Games", true, true),
            profile(12, "GOG", false, false),
        ];
        let menu = game_menu(game, &profiles, &texts);
        assert_eq!(menu.len(), 5);
        assert_eq!(
            item(&menu[0]),
            (
                "Open game page",
                Some("Middle click"),
                &Action::OpenGame(game),
                true,
                false
            )
        );
        assert_eq!(menu[1], MenuEntry::Separator);
        assert_eq!(
            item(&menu[2]),
            (
                "Play with Steam",
                None,
                &Action::LaunchProfile {
                    game,
                    profile: Uuid::from_u128(10)
                },
                true,
                false
            )
        );
        let (text, hint, _, enabled, default) = item(&menu[3]);
        assert_eq!((text, hint), ("Play with Epic Games", Some("Default")));
        assert!(enabled && default, "the default profile is marked");
        let (_, _, _, enabled, _) = item(&menu[4]);
        assert!(
            !enabled,
            "a profile whose copy is not installed is shown disabled"
        );
    }

    #[test]
    fn a_single_profile_is_still_listed() {
        let game = Uuid::from_u128(1);
        let menu = game_menu(
            game,
            &[profile(10, "Steam", true, true)],
            &PopupTexts::default(),
        );
        assert_eq!(menu.len(), 3);
        assert_eq!(
            item(&menu[2]).2,
            &Action::LaunchProfile {
                game,
                profile: Uuid::from_u128(10)
            }
        );
    }

    #[test]
    fn without_profiles_plain_play_is_offered() {
        let game = Uuid::from_u128(1);
        let menu = game_menu(game, &[], &PopupTexts::default());
        assert_eq!(menu.len(), 3);
        assert_eq!(item(&menu[2]).0, "Play");
        assert_eq!(item(&menu[2]).2, &Action::Launch(game));
    }

    #[test]
    fn native_text_escapes_ampersands_and_puts_the_hint_in_the_shortcut_column() {
        assert_eq!(native_text("Tom & Jerry", None), "Tom && Jerry");
        assert_eq!(
            native_text("Open game page", Some("Middle click")),
            "Open game page\tMiddle click"
        );
    }
}
