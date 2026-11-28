//! Requests from outside the app: `questkeep://` links and command-line arguments that start a
//! game, open its page or search the library (Stream Deck, Steam shortcuts, desktop shortcuts).
//!
//! - `questkeep://start/<game id>` (optionally `?profile=<launch profile id>`), `--start <id>`
//!   (with `--profile <id>`)
//! - `questkeep://show/<game id>`, `--show <id>`
//! - `questkeep://search/<text>`, `--search <text>`

use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

/// The URI scheme Questkeep registers.
pub const URI_SCHEME: &str = "questkeep";

/// What a link or the command line asks for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExternalRequest {
    /// Start the game (its default launch profile unless one is given).
    Start { game: Uuid, profile: Option<Uuid> },
    /// Open the game's page.
    Show { game: Uuid },
    /// Open Find a game with this text.
    Search { text: String },
}

/// A game's ID and the two ways to start it from outside, for the game page's copy menu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameLinks {
    pub id: String,
    /// `questkeep://start/<id>`
    pub url: String,
    /// The executable with `--start <id>`, quoted for a shell or a shortcut.
    pub command: String,
}

/// The ID, start link and start command of `game`; `exe` is the running executable.
pub fn game_links(game: Uuid, exe: &Path) -> GameLinks {
    GameLinks {
        id: game.to_string(),
        url: format!("{URI_SCHEME}://start/{game}"),
        command: format!("\"{}\" --start {game}", exe.display()),
    }
}

/// The request in a process's arguments (the program itself may be among them): a
/// `questkeep://` link, or `--start`, `--show` or `--search` with its value. `None` when there is
/// none, or it is malformed (an ID that is not one, an empty search).
pub fn parse_request<I, S>(args: I) -> Option<ExternalRequest>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args: Vec<String> = args.into_iter().map(|a| a.as_ref().to_string()).collect();
    if let Some(uri) = args.iter().find(|a| has_scheme(a)) {
        return parse_uri(uri);
    }
    let value = |flag: &str| {
        args.iter()
            .position(|a| a.eq_ignore_ascii_case(flag))
            .and_then(|at| args.get(at + 1))
            .map(String::as_str)
    };
    if let Some(id) = value("--start") {
        let profile = match value("--profile") {
            Some(profile) => Some(Uuid::parse_str(profile).ok()?),
            None => None,
        };
        return Some(ExternalRequest::Start {
            game: Uuid::parse_str(id).ok()?,
            profile,
        });
    }
    if let Some(id) = value("--show") {
        return Some(ExternalRequest::Show {
            game: Uuid::parse_str(id).ok()?,
        });
    }
    let text = value("--search")?.trim();
    (!text.is_empty()).then(|| ExternalRequest::Search {
        text: text.to_string(),
    })
}

fn has_scheme(arg: &str) -> bool {
    arg.len() > URI_SCHEME.len()
        && arg.is_char_boundary(URI_SCHEME.len())
        && arg[..URI_SCHEME.len()].eq_ignore_ascii_case(URI_SCHEME)
        && arg[URI_SCHEME.len()..].starts_with(':')
}

/// Parses a `questkeep://action/value?query` link (case-insensitive scheme and action; a
/// trailing slash, which some launchers add, is ignored).
pub fn parse_uri(uri: &str) -> Option<ExternalRequest> {
    if !has_scheme(uri) {
        return None;
    }
    let rest = uri[URI_SCHEME.len() + 1..].trim_start_matches('/');
    let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
    let path = path.trim_end_matches('/');
    let (action, value) = path.split_once('/').unwrap_or((path, ""));
    let value = percent_decode(value)?;
    match action.to_ascii_lowercase().as_str() {
        "start" => {
            let profile = match query_value(query, "profile") {
                Some(profile) => Some(Uuid::parse_str(&profile).ok()?),
                None => None,
            };
            Some(ExternalRequest::Start {
                game: Uuid::parse_str(&value).ok()?,
                profile,
            })
        }
        "show" => Some(ExternalRequest::Show {
            game: Uuid::parse_str(&value).ok()?,
        }),
        "search" => {
            let text = value.trim();
            (!text.is_empty()).then(|| ExternalRequest::Search {
                text: text.to_string(),
            })
        }
        _ => None,
    }
}

fn query_value(query: &str, key: &str) -> Option<String> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .and_then(|(_, v)| percent_decode(v))
}

/// Decodes `%XX` escapes (and `+` as a space); `None` for a broken escape or invalid UTF-8.
fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while let Some(&byte) = bytes.get(i) {
        match byte {
            b'%' => {
                let hex = bytes.get(i + 1..i + 3)?;
                let hex = std::str::from_utf8(hex).ok()?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAME: &str = "0e1a7dec-52e5-4892-abaf-5c82e35d314a";
    const PROFILE: &str = "5110df6f-cae6-4499-9e4e-fdc9e6cec778";

    fn id(text: &str) -> Uuid {
        Uuid::parse_str(text).unwrap()
    }

    fn start(profile: Option<&str>) -> Option<ExternalRequest> {
        Some(ExternalRequest::Start {
            game: id(GAME),
            profile: profile.map(id),
        })
    }

    #[test]
    fn parses_start_show_and_search_links() {
        assert_eq!(parse_uri(&format!("questkeep://start/{GAME}")), start(None));
        assert_eq!(
            parse_uri(&format!("questkeep://start/{GAME}?profile={PROFILE}")),
            start(Some(PROFILE))
        );
        assert_eq!(
            parse_uri(&format!("questkeep://show/{GAME}")),
            Some(ExternalRequest::Show { game: id(GAME) })
        );
        assert_eq!(
            parse_uri("questkeep://search/Hollow%20Knight"),
            Some(ExternalRequest::Search {
                text: "Hollow Knight".into()
            })
        );
        assert_eq!(
            parse_uri("questkeep://search/caf%C3%A9+racer"),
            Some(ExternalRequest::Search {
                text: "café racer".into()
            })
        );
    }

    #[test]
    fn links_are_forgiving_about_case_and_slashes() {
        assert_eq!(
            parse_uri(&format!("QuestKeep://START/{GAME}/")),
            start(None)
        );
        assert_eq!(parse_uri(&format!("questkeep:start/{GAME}")), start(None));
    }

    #[test]
    fn rejects_malformed_links() {
        assert_eq!(parse_uri("questkeep://start/not-an-id"), None);
        assert_eq!(
            parse_uri(&format!("questkeep://start/{GAME}?profile=x")),
            None
        );
        assert_eq!(parse_uri(&format!("questkeep://uninstall/{GAME}")), None);
        assert_eq!(parse_uri("questkeep://search/"), None);
        assert_eq!(parse_uri("questkeep://search/%zz"), None);
        assert_eq!(parse_uri("questkeep://search/%ff"), None);
        assert_eq!(parse_uri("https://example.com"), None);
        assert_eq!(parse_uri("questkeeper://show/x"), None);
    }

    #[test]
    fn parses_command_lines() {
        let exe = r"C:\Games\Questkeep\questkeep.exe";
        assert_eq!(parse_request([exe, "--start", GAME]), start(None));
        assert_eq!(
            parse_request([exe, "--profile", PROFILE, "--START", GAME]),
            start(Some(PROFILE))
        );
        assert_eq!(
            parse_request([exe, "--show", GAME]),
            Some(ExternalRequest::Show { game: id(GAME) })
        );
        assert_eq!(
            parse_request([exe, "--search", "  hades "]),
            Some(ExternalRequest::Search {
                text: "hades".into()
            })
        );
        // A link handed over by the OS ("questkeep.exe" "questkeep://...").
        assert_eq!(
            parse_request([exe.to_string(), format!("questkeep://start/{GAME}")]),
            start(None)
        );
    }

    #[test]
    fn ignores_other_and_malformed_arguments() {
        assert_eq!(parse_request(["questkeep.exe"]), None);
        assert_eq!(parse_request(["questkeep.exe", "--minimized"]), None);
        assert_eq!(parse_request(["questkeep.exe", "--start"]), None);
        assert_eq!(parse_request(["questkeep.exe", "--start", "hades"]), None);
        assert_eq!(
            parse_request(["questkeep.exe", "--start", GAME, "--profile", "x"]),
            None
        );
        assert_eq!(parse_request(["questkeep.exe", "--search", "  "]), None);
    }

    #[test]
    fn builds_the_links_of_a_game() {
        let links = game_links(
            id(GAME),
            Path::new(r"C:\Program Files\Questkeep\questkeep.exe"),
        );
        assert_eq!(links.id, GAME);
        assert_eq!(links.url, format!("questkeep://start/{GAME}"));
        assert_eq!(
            links.command,
            format!(r#""C:\Program Files\Questkeep\questkeep.exe" --start {GAME}"#)
        );
        // The link it builds is one it understands.
        assert_eq!(parse_uri(&links.url), start(None));
    }
}
