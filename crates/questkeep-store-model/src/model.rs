//! What stores report: games, how to start them, their metadata. Plain serde data shared with the
//! UI through ts-rs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Store metadata for one game, as returned by a provider. Every field is optional: a store fills
/// what it knows, the library picks per field from the sources in the user's priority order.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct GameMetadata {
    /// Plain text (providers strip HTML).
    pub description: Option<String>,
    /// `YYYY-MM-DD`, `YYYY-MM` or `YYYY`.
    pub release_date: Option<String>,
    pub developers: Vec<String>,
    pub publishers: Vec<String>,
    pub genres: Vec<String>,
    /// User or store tags ("Roguelike", "Great Soundtrack", ...).
    pub tags: Vec<String>,
    /// What the game supports: "Single-player", "Co-op", "Controller support", "Cloud saves", ...
    pub features: Vec<String>,
    /// Franchise or series names.
    pub series: Vec<String>,
    /// "PEGI 16", "ESRB Teen", "USK 12", ...
    pub age_ratings: Vec<String>,
    /// "Windows", "macOS", "Linux", console names.
    pub platforms: Vec<String>,
    pub game_modes: Vec<String>,
    pub themes: Vec<String>,
    pub perspectives: Vec<String>,
    /// Press score, 0-100.
    pub critic_score: Option<Score>,
    /// Player score, 0-100 (e.g. the share of positive reviews).
    pub community_score: Option<Score>,
    #[ts(type = "number | null")]
    pub install_size: Option<u64>,
    pub time_to_beat: Option<TimeToBeat>,
    /// Pages about the game (store page, wiki, community, socials). Merged across sources.
    pub links: Vec<GameLink>,
}

/// A 0-100 score and who gave it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Score {
    pub value: u8,
    /// Shown next to the score, e.g. `Metacritic` or `Steam reviews`.
    pub source: String,
}

/// How long the game takes, in seconds.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct TimeToBeat {
    #[ts(type = "number | null")]
    pub main: Option<u64>,
    #[ts(type = "number | null")]
    pub extra: Option<u64>,
    #[ts(type = "number | null")]
    pub complete: Option<u64>,
}

impl TimeToBeat {
    pub fn is_empty(&self) -> bool {
        self.main.is_none() && self.extra.is_none() && self.complete.is_none()
    }
}

/// A web page about a game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameLink {
    pub name: String,
    pub url: String,
}

impl GameLink {
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
        }
    }
}

/// What a plugin store tells about its account: the app adds the store's id and name to make the
/// status the settings page shows. Only meaningful for stores with `permissions.login`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AccountInfo {
    pub signed_in: bool,
    pub display_name: Option<String>,
    /// Whether the user can paste a code (or address) instead of using the sign-in window.
    pub supports_code_sign_in: bool,
    /// Page the user opens in their normal browser to obtain the code.
    pub code_url: Option<String>,
    /// What to copy from the store's page after signing in there.
    pub code_hint: Option<String>,
}

/// How a game is started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ActionKind {
    /// Start an executable.
    File,
    /// Open a URI with the system handler (e.g. `steam://rungameid/440`).
    Uri,
}

/// How the launcher decides that a game is still running.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TrackingMode {
    /// Process tree for `file` actions, install directory for `uri` actions.
    #[default]
    Default,
    /// The started process and all of its descendants.
    Process,
    /// Only the started process.
    OriginalProcess,
    /// Any process whose executable lives under `tracking_path` (or the install dir).
    Directory,
    /// Any process whose executable file name equals `tracking_path`.
    Name,
}

/// The single play action of a game. Stored as JSON so it can grow into a list later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlayAction {
    pub kind: ActionKind,
    /// Executable path for `file`, URI for `uri`.
    pub target: String,
    /// Command-line arguments as typed by the user.
    #[serde(default)]
    pub args: Option<String>,
    #[serde(default)]
    pub working_dir: Option<String>,
    #[serde(default)]
    pub tracking: TrackingMode,
    #[serde(default)]
    pub tracking_path: Option<String>,
}

impl PlayAction {
    pub fn file(target: impl Into<String>) -> Self {
        Self {
            kind: ActionKind::File,
            target: target.into(),
            args: None,
            working_dir: None,
            tracking: TrackingMode::Default,
            tracking_path: None,
        }
    }

    pub fn uri(target: impl Into<String>) -> Self {
        Self {
            kind: ActionKind::Uri,
            ..Self::file(target)
        }
    }
}

/// The store's own launcher as a running program, so the app can close it after a game (the
/// "Close launcher after playing" store setting).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LauncherProcess {
    /// Executable file names (e.g. `steam.exe`, matched case-insensitively) whose running
    /// process means the launcher is open.
    pub process_names: Vec<String>,
    pub close: LauncherClose,
}

/// How the launcher is closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
#[ts(export)]
pub enum LauncherClose {
    /// The launcher's own clean exit (e.g. `steam.exe -shutdown`). Never followed by force.
    Command(PlayAction),
    /// Ask the processes' windows to close (Linux: SIGTERM), then end them by force if they
    /// still run after a grace period. `process_names` empty means all of
    /// [`LauncherProcess::process_names`].
    CloseWindows { process_names: Vec<String> },
    /// End the processes right away; for launchers with no clean way to quit.
    ForceQuit,
}

/// A game as reported by a library provider scan. Plain data so providers can later run as
/// sandboxed plugins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportedGame {
    pub source_game_id: String,
    pub name: String,
    /// `false` for games that are owned but not installed (from `scan_owned`).
    #[serde(default = "default_true")]
    pub installed: bool,
    pub install_dir: Option<String>,
    pub play_action: Option<PlayAction>,
    /// Playtime known to the store. Merged as the maximum with Questkeep's own tracking.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub playtime_seconds: Option<u64>,
    /// Last played according to the store. Merged as the later of both values.
    #[serde(default)]
    pub last_played: Option<DateTime<Utc>>,
    /// Whether the store lists it as an app (software, tool) rather than a game; `None` when
    /// the store doesn't say (the entry keeps what it had, new ones are games).
    #[serde(default)]
    pub app: Option<bool>,
}

fn default_true() -> bool {
    true
}

impl ImportedGame {
    /// An installed game with the required fields; everything else empty.
    pub fn installed(source_game_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            source_game_id: source_game_id.into(),
            name: name.into(),
            installed: true,
            install_dir: None,
            play_action: None,
            playtime_seconds: None,
            last_played: None,
            app: None,
        }
    }
}

/// A news item about a game (game page).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NewsItem {
    pub title: String,
    pub url: String,
    /// Unix seconds.
    #[ts(type = "number | null")]
    pub date: Option<i64>,
    /// Who wrote it, e.g. "Steam Community Announcements" or "PC Gamer".
    pub source: String,
    /// Plain text, markup removed, at most a few hundred characters.
    pub snippet: String,
    /// Image URL, when the item has one.
    pub image: Option<String>,
}

/// The latest news of a game.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameNews {
    /// Newest first.
    pub items: Vec<NewsItem>,
    /// The store's page with all news for the game.
    pub all_url: Option<String>,
}

/// An achievement the user unlocked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Achievement {
    pub name: String,
    pub description: Option<String>,
    /// Icon URL.
    pub icon: Option<String>,
    /// Unix seconds; `None` when the store doesn't say.
    #[ts(type = "number | null")]
    pub unlocked_at: Option<i64>,
    /// Share of all players that have it (0-100), when known.
    pub percent: Option<f32>,
}

/// The user's progress in a game's achievements.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Achievements {
    /// How many the game has.
    pub total: u32,
    /// The unlocked ones, most recent first.
    pub unlocked: Vec<Achievement>,
}

/// An entry a store adds to the menu of the "Add game" button: a way to add games, the store's
/// shop, or where to redeem a product key.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AddGameOption {
    /// Stable within the provider.
    pub id: String,
    /// Menu text, e.g. `New Minecraft instance`. For [`buy`](AddGameSection::Buy) and
    /// [`activate`](AddGameSection::Activate) options the store's name: it follows "Buy from" or
    /// "Activate on".
    pub label: String,
    /// The submenu it is listed in.
    #[serde(default)]
    pub section: AddGameSection,
    /// What choosing it does. After an [`add`](AddGameSection::Add) option Questkeep watches the
    /// store's installed games for a few minutes and adds the new one by itself.
    #[serde(default)]
    pub opens: AddGameTarget,
}

/// The submenu of the "Add game" menu an [`AddGameOption`] is listed in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AddGameSection {
    /// "Add": a way to add a game, e.g. the launcher's "new instance" dialog.
    #[default]
    Add,
    /// "Buy from": the store's shop.
    Buy,
    /// "Activate on": where the store redeems a product key.
    Activate,
}

/// What an [`AddGameOption`] opens.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum AddGameTarget {
    /// The provider's own `run_add_game_option` (built-in stores).
    #[default]
    Provider,
    /// The store's launcher (what the provider's `launcher()` starts) with `args` added, e.g.
    /// `--show-window`; empty arguments open it as it is. This is how a store plugin, which
    /// cannot start programs itself, opens its launcher for adding a game. Optionally presses
    /// `keys` in it, for a dialog that has no command-line option.
    Launcher {
        args: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        keys: Option<LauncherKeys>,
    },
    /// A web page (`https://` only) in the user's browser, e.g. a site to get games from.
    Browser { url: String },
    /// A store URI the system opens, e.g. `steam://open/activateproduct`. Built-in stores only:
    /// options of store plugins that use it are dropped.
    Uri { uri: String },
}

/// Key presses in the launcher's window after an add-game option opened it (Windows only).
///
/// Questkeep waits up to 20 seconds for a window of `process` whose title starts with
/// `window_title` to come to the front. Then it goes through `steps` in order: it waits the
/// step's delay, checks that a window of `process` is still in front (a dialog the previous step
/// opened counts, whatever its title) and presses the step's keys. When another program is in
/// front it stops: nothing is ever typed into another program. A store plugin must list `process`
/// in `permissions.launcher.processNames`, otherwise the keys are dropped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LauncherKeys {
    /// The launcher's process file name, e.g. `prismlauncher.exe`.
    pub process: String,
    /// The start of the launcher window's title, e.g. `Prism Launcher`.
    pub window_title: String,
    /// Pressed in order; at most [`LauncherKeys::MAX_STEPS`].
    pub steps: Vec<KeyStep>,
    /// A [`wait_ms`](LauncherKeys::wait_ms) that the next [`press`](LauncherKeys::press) takes.
    #[serde(skip)]
    #[ts(skip)]
    pending_delay_ms: u32,
}

/// One key combination of [`LauncherKeys`] and the wait before it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct KeyStep {
    /// A combination like `Mod+N`, `Ctrl+Shift+B`, `Alt+R` or `Enter` (see [`KeyCombo::parse`]).
    pub keys: String,
    /// Milliseconds to wait before pressing, at most [`LauncherKeys::MAX_DELAY_MS`].
    #[serde(default)]
    pub delay_ms: u32,
}

impl LauncherKeys {
    /// The longest wait before one step (5 seconds).
    pub const MAX_DELAY_MS: u32 = 5000;
    /// The most steps one option may press.
    pub const MAX_STEPS: usize = 16;

    /// Keys for the window of `process` titled `window_title…`; add steps with
    /// [`wait_ms`](Self::wait_ms) and [`press`](Self::press).
    pub fn new(process: impl Into<String>, window_title: impl Into<String>) -> Self {
        Self {
            process: process.into(),
            window_title: window_title.into(),
            steps: Vec::new(),
            pending_delay_ms: 0,
        }
    }

    /// Waits `ms` milliseconds before the next [`press`](Self::press) (two waits add up).
    pub fn wait_ms(mut self, ms: u32) -> Self {
        self.pending_delay_ms = self.pending_delay_ms.saturating_add(ms);
        self
    }

    /// Then presses `keys`, e.g. `Ctrl+Shift+B`.
    pub fn press(mut self, keys: impl Into<String>) -> Self {
        self.steps.push(KeyStep {
            keys: keys.into(),
            delay_ms: std::mem::take(&mut self.pending_delay_ms),
        });
        self
    }

    /// The steps as delays and combinations parsed for `platform`, or what is wrong with them (an
    /// unknown key, a modifier that platform doesn't have, a wait over 5 seconds, no steps or too
    /// many).
    pub fn parsed(&self, platform: KeyPlatform) -> Result<Vec<(u32, KeyCombo)>, String> {
        if self.steps.is_empty() {
            return Err("no keys to press".into());
        }
        if self.steps.len() > Self::MAX_STEPS {
            return Err(format!("more than {} key steps", Self::MAX_STEPS));
        }
        self.steps
            .iter()
            .map(|step| {
                if step.delay_ms > Self::MAX_DELAY_MS {
                    return Err(format!(
                        "the wait before \"{}\" is longer than {} ms",
                        step.keys,
                        Self::MAX_DELAY_MS
                    ));
                }
                Ok((step.delay_ms, KeyCombo::parse(&step.keys, platform)?))
            })
            .collect()
    }
}

/// The operating system key combinations are parsed for: it decides what `Mod` means and which
/// modifiers exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPlatform {
    Windows,
    Linux,
    MacOs,
}

impl KeyPlatform {
    /// The system this code was compiled for (Linux for anything that isn't Windows or macOS,
    /// such as a WASM plugin, which asks `system::platform()` instead).
    pub fn current() -> Self {
        if cfg!(windows) {
            KeyPlatform::Windows
        } else if cfg!(target_os = "macos") {
            KeyPlatform::MacOs
        } else {
            KeyPlatform::Linux
        }
    }
}

/// A parsed key combination for one [`KeyPlatform`]: modifiers plus one key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyCombo {
    pub ctrl: bool,
    pub shift: bool,
    /// Alt (Option on a Mac).
    pub alt: bool,
    /// The Windows key (Super on Linux). Never set on macOS.
    pub super_key: bool,
    /// The Mac Command key. Only set on macOS.
    pub cmd: bool,
    pub key: Key,
}

/// The key a [`KeyCombo`] presses besides its modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// `A`–`Z`, upper case.
    Letter(char),
    /// `0`–`9`.
    Digit(char),
    /// `F1`–`F24`.
    Function(u8),
    Enter,
    Tab,
    Escape,
    Space,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
}

impl KeyCombo {
    /// Parses `Mod+Shift+B` for `platform`: modifiers, then exactly one key. Case and spaces
    /// around `+` don't matter.
    ///
    /// - Modifiers: `Mod` (Ctrl on Windows and Linux, Cmd on macOS: the key most shortcuts use),
    ///   `Ctrl`/`Control`, `Shift`, `Alt`/`Option`, `Win`/`Super` (Windows and Linux only) and
    ///   `Cmd`/`Command` (macOS only). A modifier the platform doesn't have is an error, so a
    ///   shortcut meant for one system is never pressed as something else on another.
    /// - Keys: a letter, a digit, `F1`–`F24`, `Enter`, `Tab`, `Esc`, `Space`, `Backspace`,
    ///   `Delete`, `Insert`, `Home`, `End`, `PageUp`, `PageDown` or an arrow (`Up`, `Down`,
    ///   `Left`, `Right`).
    pub fn parse(text: &str, platform: KeyPlatform) -> Result<Self, String> {
        let mac = platform == KeyPlatform::MacOs;
        let (mut ctrl, mut shift, mut alt, mut super_key, mut cmd) =
            (false, false, false, false, false);
        let mut key = None;
        for part in text.split('+').map(str::trim) {
            let lower = part.to_ascii_lowercase();
            let modifier = match lower.as_str() {
                "mod" if mac => Some(&mut cmd),
                "mod" | "ctrl" | "control" => Some(&mut ctrl),
                "shift" => Some(&mut shift),
                "alt" | "option" => Some(&mut alt),
                "win" | "windows" | "super" if mac => {
                    return Err(format!(
                        "\"{text}\": macOS has no {part} key (Mod is Cmd there)"
                    ));
                }
                "win" | "windows" | "super" => Some(&mut super_key),
                "cmd" | "command" if mac => Some(&mut cmd),
                "cmd" | "command" => {
                    return Err(format!(
                        "\"{text}\": {part} is the Mac key; Mod is Ctrl on Windows and Linux"
                    ));
                }
                _ => None,
            };
            if let Some(flag) = modifier {
                if key.is_some() {
                    return Err(format!("\"{text}\": modifiers come before the key"));
                }
                *flag = true;
                continue;
            }
            if key.is_some() {
                return Err(format!("\"{text}\" has more than one key"));
            }
            key = Some(Key::parse(&lower).ok_or_else(|| format!("unknown key \"{part}\""))?);
        }
        Ok(KeyCombo {
            ctrl,
            shift,
            alt,
            super_key,
            cmd,
            key: key.ok_or_else(|| format!("\"{text}\" has no key"))?,
        })
    }
}

impl Key {
    fn parse(lower: &str) -> Option<Self> {
        let mut chars = lower.chars();
        if let (Some(c), None) = (chars.next(), chars.clone().next()) {
            return match c {
                'a'..='z' => Some(Key::Letter(c.to_ascii_uppercase())),
                '0'..='9' => Some(Key::Digit(c)),
                _ => None,
            };
        }
        if let Some(n) = lower.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()) {
            return (1..=24).contains(&n).then_some(Key::Function(n));
        }
        Some(match lower {
            "enter" | "return" => Key::Enter,
            "tab" => Key::Tab,
            "esc" | "escape" => Key::Escape,
            "space" => Key::Space,
            "backspace" => Key::Backspace,
            "delete" | "del" => Key::Delete,
            "insert" | "ins" => Key::Insert,
            "home" => Key::Home,
            "end" => Key::End,
            "pageup" | "pgup" => Key::PageUp,
            "pagedown" | "pgdn" => Key::PageDown,
            "up" => Key::Up,
            "down" => Key::Down,
            "left" => Key::Left,
            "right" => Key::Right,
            _ => return None,
        })
    }
}

impl AddGameOption {
    /// An option handled by the provider's `run_add_game_option`.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            section: AddGameSection::Add,
            opens: AddGameTarget::Provider,
        }
    }

    /// A "Buy from" entry; `store_name` follows "Buy from". Give it a target, e.g.
    /// [`opening_browser`](Self::opening_browser).
    pub fn buy(id: impl Into<String>, store_name: impl Into<String>) -> Self {
        Self {
            section: AddGameSection::Buy,
            ..Self::new(id, store_name)
        }
    }

    /// An "Activate on" entry (where a product key is redeemed); `store_name` follows "Activate
    /// on". Give it a target, e.g. [`opening_browser`](Self::opening_browser).
    pub fn activate(id: impl Into<String>, store_name: impl Into<String>) -> Self {
        Self {
            section: AddGameSection::Activate,
            ..Self::new(id, store_name)
        }
    }

    /// Opens the store URI `uri` (built-in stores only, see [`AddGameTarget::Uri`]).
    pub fn opening_uri(mut self, uri: impl Into<String>) -> Self {
        self.opens = AddGameTarget::Uri { uri: uri.into() };
        self
    }

    /// Whether choosing it only opens a web page, which works without the store's client.
    pub fn opens_browser(&self) -> bool {
        matches!(self.opens, AddGameTarget::Browser { .. })
    }

    /// Opens the store's launcher with `args` added instead of calling `run_add_game_option`.
    pub fn opening_launcher(mut self, args: impl Into<String>) -> Self {
        self.opens = AddGameTarget::Launcher {
            args: args.into(),
            keys: None,
        };
        self
    }

    /// Opens the store's launcher with `args` added, then presses `keys` in its window.
    pub fn opening_launcher_with_keys(
        mut self,
        args: impl Into<String>,
        keys: LauncherKeys,
    ) -> Self {
        self.opens = AddGameTarget::Launcher {
            args: args.into(),
            keys: Some(keys),
        };
        self
    }

    /// Opens `url` (`https://` only) in the user's browser instead of calling
    /// `run_add_game_option`.
    pub fn opening_browser(mut self, url: impl Into<String>) -> Self {
        self.opens = AddGameTarget::Browser { url: url.into() };
        self
    }
}

#[cfg(test)]
mod add_game_option_tests {
    use super::*;

    #[test]
    fn key_combos_parse_modifiers_and_one_key() {
        use KeyPlatform::{Linux, MacOs, Windows};
        let combo = KeyCombo::parse("Ctrl + Shift+b", Windows).unwrap();
        assert!(combo.ctrl && combo.shift && !combo.alt && !combo.super_key && !combo.cmd);
        assert_eq!(combo.key, Key::Letter('B'));
        let combo = KeyCombo::parse("super+alt+F12", Linux).unwrap();
        assert!(combo.super_key && combo.alt && !combo.ctrl);
        assert_eq!(combo.key, Key::Function(12));
        assert_eq!(KeyCombo::parse("Enter", MacOs).unwrap().key, Key::Enter);
        assert_eq!(
            KeyCombo::parse("Win+7", Windows).unwrap().key,
            Key::Digit('7')
        );
        assert!(KeyCombo::parse("Option+X", MacOs).unwrap().alt);
        for bad in [
            "",
            "Ctrl",
            "Ctrl+A+B",
            "A+Ctrl",
            "Ctrl+ä",
            "F25",
            "F0",
            "Ctrl+Plus",
            "Hyper+A",
            "Meta+A",
        ] {
            assert!(
                KeyCombo::parse(bad, Windows).is_err(),
                "{bad} must not parse"
            );
        }
    }

    #[test]
    fn mod_is_the_shortcut_key_of_each_platform() {
        use KeyPlatform::{Linux, MacOs, Windows};
        for platform in [Windows, Linux] {
            let combo = KeyCombo::parse("Mod+N", platform).unwrap();
            assert!(combo.ctrl && !combo.cmd, "{platform:?}");
            assert!(
                KeyCombo::parse("Cmd+N", platform).is_err(),
                "Cmd is Mac only"
            );
        }
        let combo = KeyCombo::parse("Mod+N", MacOs).unwrap();
        assert!(combo.cmd && !combo.ctrl);
        assert_eq!(KeyCombo::parse("Command+N", MacOs).unwrap(), combo);
        assert!(
            KeyCombo::parse("Ctrl+N", MacOs).unwrap().ctrl,
            "the Control key"
        );
        assert!(KeyCombo::parse("Win+N", MacOs).is_err());
        assert!(KeyCombo::parse("Super+N", MacOs).is_err());
    }

    #[test]
    fn launcher_keys_check_delays_and_steps() {
        let keys = LauncherKeys::new("a.exe", "A")
            .wait_ms(500)
            .press("Ctrl+Shift+B")
            .wait_ms(3000)
            .wait_ms(2000)
            .press("Alt+R");
        assert_eq!(
            keys.parsed(KeyPlatform::Windows)
                .unwrap()
                .iter()
                .map(|s| s.0)
                .collect::<Vec<_>>(),
            [500, 5000],
            "waits add up"
        );
        let wait = |ms| LauncherKeys::new("a.exe", "A").wait_ms(ms).press("A");
        assert!(wait(5001).parsed(KeyPlatform::Windows).is_err());
        assert!(
            LauncherKeys::new("a.exe", "A")
                .parsed(KeyPlatform::Windows)
                .is_err(),
            "no keys"
        );
        assert!(
            LauncherKeys::new("a.exe", "A")
                .press("Ctrl+?")
                .parsed(KeyPlatform::Windows)
                .is_err()
        );
        let many = (0..=LauncherKeys::MAX_STEPS)
            .fold(LauncherKeys::new("a.exe", "A"), |keys, _| keys.press("A"));
        assert!(many.parsed(KeyPlatform::Windows).is_err());
    }

    #[test]
    fn add_game_options_without_a_target_ask_the_provider() {
        let option: AddGameOption = serde_json::from_str(r#"{"id":"x","label":"X"}"#).unwrap();
        assert_eq!(option.opens, AddGameTarget::Provider);
        assert_eq!(
            option.section,
            AddGameSection::Add,
            "older plugins add games"
        );
        let json = serde_json::to_value(AddGameOption::new("x", "X").opening_launcher_with_keys(
            "--show-window",
            LauncherKeys::new("p.exe", "P").wait_ms(100).press("Ctrl+N"),
        ))
        .unwrap();
        assert_eq!(
            json["opens"],
            serde_json::json!({
                "kind": "launcher",
                "args": "--show-window",
                "keys": {
                    "process": "p.exe",
                    "windowTitle": "P",
                    "steps": [{ "keys": "Ctrl+N", "delayMs": 100 }]
                }
            })
        );
        let browser: AddGameOption = serde_json::from_value(serde_json::json!({
            "id": "web", "label": "Web", "opens": { "kind": "browser", "url": "https://example.com" }
        }))
        .unwrap();
        assert_eq!(
            browser.opens,
            AddGameTarget::Browser {
                url: "https://example.com".into()
            }
        );
    }

    #[test]
    fn buy_and_activate_options_name_their_section() {
        let buy = AddGameOption::buy("shop", "Steam").opening_browser("https://example.com");
        assert_eq!(buy.section, AddGameSection::Buy);
        assert!(buy.opens_browser());
        let activate = AddGameOption::activate("key", "Steam").opening_uri("steam://open/x");
        let json = serde_json::to_value(&activate).unwrap();
        assert_eq!(json["section"], "activate");
        assert_eq!(
            json["opens"],
            serde_json::json!({ "kind": "uri", "uri": "steam://open/x" })
        );
        assert!(!activate.opens_browser());
    }
}

/// A game found by searching a store's catalog by name, so a game added by hand can get its
/// details and images from that store (see `LibraryProvider::search_catalog`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CatalogHit {
    /// The store's id for the game, as `metadata` and the image candidates take it.
    pub id: String,
    pub name: String,
    pub release_year: Option<i32>,
    /// A small picture to tell similar names apart (a capsule, cover or logo URL).
    pub image: Option<String>,
}
