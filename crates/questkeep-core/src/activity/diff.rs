//! What of a library entry the activity log shows, and how two states of it differ. Pure: the
//! recorder builds an [`EntrySnapshot`] before and after a change and writes what [`diff`] finds.
//!
//! The field table ([`EntrySnapshot::of`]) decides how each field is shown:
//! - with values: plain fields as before → after, lists as sets (added, removed);
//! - `changed` only: images (their paths are internal);
//! - not at all: internal markers in `extra`, field sources, derived sorting names, metadata
//!   timestamps, playtime / play count / last played (sessions cover those).

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use crate::model::{Game, METADATA_LINK_KEY, METADATA_NAME_KEY};

use super::EntryKind;

/// How a change is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ChangeMode {
    /// `before` → `after` (either may be absent: set or cleared).
    Value,
    /// A list: `before` holds the removed items, `after` the added ones (arrays).
    Set,
    /// Only that it changed (images): `before`/`after` say whether there was one.
    Changed,
}

/// One field of one entry that changed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ActivityChange")]
pub struct FieldChange {
    /// Field key, see the field table (`name`, `genres`, `cover`, ...).
    pub field: String,
    pub mode: ChangeMode,
    #[ts(type = "unknown")]
    pub before: Option<Value>,
    #[ts(type = "unknown")]
    pub after: Option<Value>,
}

impl FieldChange {
    pub fn value(field: &str, before: Option<Value>, after: Option<Value>) -> Self {
        Self {
            field: field.to_string(),
            mode: ChangeMode::Value,
            before: before.filter(|v| !v.is_null()),
            after: after.filter(|v| !v.is_null()),
        }
    }
}

/// One field's state in a snapshot.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    Value(Value),
    /// Items by key; only keys are compared, the values are what is shown.
    Set(BTreeMap<String, Value>),
    /// Something internal (an image path): compared, never shown.
    Opaque(Option<String>),
}

/// A launch profile as the log shows it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProfileFacts {
    pub id: String,
    pub name: Option<String>,
    pub args: Option<String>,
    /// What it starts (own profiles).
    pub target: Option<String>,
}

/// What a snapshot needs besides the [`Game`].
#[derive(Debug, Clone, Default)]
pub struct EntryExtras {
    /// The sorting name the user set (a derived one is not shown).
    pub user_sorting_name: Option<String>,
    pub profiles: Vec<ProfileFacts>,
    /// Id of the default launch profile.
    pub default_profile: Option<String>,
}

/// The shown state of one library entry.
#[derive(Debug, Clone, PartialEq)]
pub struct EntrySnapshot {
    pub name: String,
    /// Not shown per game; a sync counts the entries whose store playtime changed.
    pub playtime: u64,
    pub fields: Vec<(&'static str, FieldValue)>,
}

fn opt<T: Serialize>(value: &Option<T>) -> FieldValue {
    FieldValue::Value(
        value
            .as_ref()
            .and_then(|v| serde_json::to_value(v).ok())
            .unwrap_or(Value::Null),
    )
}

fn plain<T: Serialize>(value: &T) -> FieldValue {
    FieldValue::Value(serde_json::to_value(value).unwrap_or(Value::Null))
}

fn strings(list: &[String]) -> FieldValue {
    FieldValue::Set(
        list.iter()
            .map(|s| (s.clone(), Value::String(s.clone())))
            .collect(),
    )
}

/// The list fields of [`Game`] shown as sets, with their keys.
const LIST_FIELDS: [&str; 12] = [
    "developers",
    "publishers",
    "genres",
    "tags",
    "features",
    "series",
    "ageRatings",
    "platforms",
    "gameModes",
    "themes",
    "perspectives",
    "regions",
];

impl EntrySnapshot {
    /// The field table: what the log shows of `game`. `categories` maps category ids to names.
    pub fn of(game: &Game, categories: &HashMap<String, String>, extras: &EntryExtras) -> Self {
        let lists: [&Vec<String>; 12] = [
            &game.developers,
            &game.publishers,
            &game.genres,
            &game.tags,
            &game.features,
            &game.series,
            &game.age_ratings,
            &game.platforms,
            &game.game_modes,
            &game.themes,
            &game.perspectives,
            &game.regions,
        ];
        let extra = |key: &str| {
            FieldValue::Value(
                game.extra
                    .get(key)
                    .cloned()
                    .filter(|v| !matches!(v, Value::String(s) if s.trim().is_empty()))
                    .unwrap_or(Value::Null),
            )
        };
        let mut fields: Vec<(&'static str, FieldValue)> = vec![
            ("name", plain(&game.name)),
            ("sortingName", opt(&extras.user_sorting_name)),
            ("kind", plain(&game.kind)),
            ("description", opt(&game.description)),
            ("releaseDate", opt(&game.release_date)),
            ("version", opt(&game.version)),
            ("manual", opt(&game.manual)),
        ];
        fields.extend(LIST_FIELDS.iter().zip(lists).map(|(k, l)| (*k, strings(l))));
        fields.extend([
            ("criticScore", opt(&game.critic_score)),
            ("communityScore", opt(&game.community_score)),
            ("timeToBeat", opt(&game.time_to_beat)),
            ("installSize", opt(&game.install_size)),
            (
                "links",
                FieldValue::Set(
                    game.links
                        .iter()
                        .map(|l| (l.url.clone(), json!({ "name": l.name, "url": l.url })))
                        .collect(),
                ),
            ),
            ("completionStatus", opt(&game.completion_status)),
            ("userScore", opt(&game.user_score)),
            ("notes", opt(&game.notes)),
            ("favorite", plain(&game.favorite)),
            ("hidden", plain(&game.hidden)),
            ("installed", plain(&game.installed)),
            ("installDir", opt(&game.install_dir)),
            ("playAction", opt(&game.play_action)),
            ("detectLaunches", plain(&game.detect_launches)),
            ("ignoredProcesses", strings(&game.ignored_processes)),
            (
                "categories",
                FieldValue::Set(
                    game.categories
                        .iter()
                        .map(|id| {
                            let name = categories.get(id).cloned().unwrap_or_else(|| id.clone());
                            (id.clone(), Value::String(name))
                        })
                        .collect(),
                ),
            ),
            (
                "copies",
                FieldValue::Set(
                    game.copies
                        .iter()
                        .map(|c| {
                            (
                                c.id.to_string(),
                                json!({
                                    "store": c.source,
                                    "storeId": c.source_game_id,
                                    "qualifier": c.qualifier,
                                }),
                            )
                        })
                        .collect(),
                ),
            ),
            // The copy Play starts first, while there is a choice.
            (
                "preferredCopy",
                FieldValue::Value(match game.copies.as_slice() {
                    [first, _, ..] => {
                        json!({ "store": first.source, "qualifier": first.qualifier })
                    }
                    _ => Value::Null,
                }),
            ),
            (
                "launchProfiles",
                FieldValue::Set(
                    extras
                        .profiles
                        .iter()
                        .map(|p| {
                            // Renamed or changed profiles show as one removed and one added.
                            let key = format!(
                                "{}\u{1f}{}\u{1f}{}\u{1f}{}",
                                p.id,
                                p.name.as_deref().unwrap_or_default(),
                                p.args.as_deref().unwrap_or_default(),
                                p.target.as_deref().unwrap_or_default()
                            );
                            (key, json!({ "name": p.name, "args": p.args }))
                        })
                        .collect(),
                ),
            ),
            (
                "defaultProfile",
                FieldValue::Value(
                    extras
                        .default_profile
                        .as_ref()
                        .map(|id| {
                            let name = extras
                                .profiles
                                .iter()
                                .find(|p| &p.id == id)
                                .and_then(|p| p.name.clone());
                            Value::String(name.unwrap_or_else(|| id.clone()))
                        })
                        .unwrap_or(Value::Null),
                ),
            ),
            ("metadataLink", extra(METADATA_LINK_KEY)),
            ("metadataName", extra(METADATA_NAME_KEY)),
            ("cover", FieldValue::Opaque(game.cover_path.clone())),
            (
                "background",
                FieldValue::Opaque(game.background_path.clone()),
            ),
            ("logo", FieldValue::Opaque(game.logo_path.clone())),
            ("icon", FieldValue::Opaque(game.icon_path.clone())),
        ]);
        Self {
            name: game.name.clone(),
            playtime: game.playtime_seconds,
            fields,
        }
    }

    fn field(&self, key: &str) -> Option<&FieldValue> {
        self.fields.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
    }
}

/// The fields that differ between two states of one entry, in field table order.
pub fn diff(before: &EntrySnapshot, after: &EntrySnapshot) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    for (key, new) in &after.fields {
        let Some(old) = before.field(key) else {
            continue;
        };
        if let Some(change) = compare(key, old, new) {
            changes.push(change);
        }
    }
    changes
}

fn compare(key: &str, old: &FieldValue, new: &FieldValue) -> Option<FieldChange> {
    match (old, new) {
        (FieldValue::Value(a), FieldValue::Value(b)) => {
            (a != b).then(|| FieldChange::value(key, Some(a.clone()), Some(b.clone())))
        }
        (FieldValue::Set(a), FieldValue::Set(b)) => {
            let removed: Vec<Value> = a
                .iter()
                .filter(|(k, _)| !b.contains_key(*k))
                .map(|(_, v)| v.clone())
                .collect();
            let added: Vec<Value> = b
                .iter()
                .filter(|(k, _)| !a.contains_key(*k))
                .map(|(_, v)| v.clone())
                .collect();
            (!removed.is_empty() || !added.is_empty()).then(|| FieldChange {
                field: key.to_string(),
                mode: ChangeMode::Set,
                before: Some(Value::Array(removed)),
                after: Some(Value::Array(added)),
            })
        }
        (FieldValue::Opaque(a), FieldValue::Opaque(b)) => (a != b).then(|| FieldChange {
            field: key.to_string(),
            mode: ChangeMode::Changed,
            before: Some(Value::Bool(a.is_some())),
            after: Some(Value::Bool(b.is_some())),
        }),
        _ => None,
    }
}

/// What an added entry shows: its name, its stores and whether it is installed.
pub fn added(after: &EntrySnapshot) -> Vec<FieldChange> {
    summary(after, false)
}

/// What a removed entry keeps, so it can be found again: its name and its stores with their ids.
pub fn removed(before: &EntrySnapshot) -> Vec<FieldChange> {
    summary(before, true)
}

fn summary(snapshot: &EntrySnapshot, gone: bool) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    for key in ["name", "copies", "installed"] {
        let Some(value) = snapshot.field(key) else {
            continue;
        };
        let (empty, value) = match value {
            FieldValue::Value(v) => (Value::Null, v.clone()),
            FieldValue::Set(items) => (
                Value::Array(Vec::new()),
                Value::Array(items.values().cloned().collect()),
            ),
            FieldValue::Opaque(_) => continue,
        };
        let (before, after) = if gone { (value, empty) } else { (empty, value) };
        let mode = if matches!(snapshot.field(key), Some(FieldValue::Set(_))) {
            ChangeMode::Set
        } else {
            ChangeMode::Value
        };
        let change = match mode {
            // A set's removed items are `before`, its added ones `after`.
            ChangeMode::Set => FieldChange {
                field: key.to_string(),
                mode,
                before: Some(before),
                after: Some(after),
            },
            _ => FieldChange::value(key, Some(before), Some(after)),
        };
        changes.push(change);
    }
    changes
}

/// The "What" of an entry whose change didn't say: from the fields that changed.
pub fn kind_of(changes: &[FieldChange]) -> EntryKind {
    let only = |keys: &[&str]| {
        !changes.is_empty() && changes.iter().all(|c| keys.contains(&c.field.as_str()))
    };
    if only(&["installed", "installDir", "playAction"]) {
        EntryKind::Install
    } else if only(&["categories"]) {
        EntryKind::Categories
    } else if only(&["cover", "background", "icon"]) {
        EntryKind::Images
    } else if only(&["completionStatus"]) {
        EntryKind::Status
    } else if only(&["launchProfiles", "defaultProfile"]) {
        EntryKind::Profiles
    } else if changes
        .iter()
        .any(|c| c.field == "copies" || c.field == "preferredCopy")
    {
        EntryKind::Combine
    } else {
        EntryKind::Metadata
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::*;
    use crate::model::{GameCopy, GameLink, PlayAction, Score};

    fn game() -> Game {
        let id = Uuid::new_v4();
        Game {
            id,
            name: "Hades".into(),
            sorting_name: "hades".into(),
            source: "steam".into(),
            source_game_id: Some("1145360".into()),
            installed: true,
            install_dir: Some("C:/Games/Hades".into()),
            play_action: Some(PlayAction::file("C:/Games/Hades/Hades.exe")),
            playtime_seconds: 60,
            play_count: 1,
            last_played: None,
            date_added: Utc::now(),
            favorite: false,
            hidden: false,
            kind: crate::model::GameKind::Game,
            detect_launches: true,
            ignored_processes: Vec::new(),
            cover_path: Some("images/covers/a.jpg".into()),
            icon_path: None,
            background_path: None,
            logo_path: None,
            description: Some("A rogue-like.".into()),
            release_date: None,
            developers: vec!["Supergiant Games".into()],
            publishers: Vec::new(),
            genres: vec!["Action".into(), "Arcade".into()],
            tags: Vec::new(),
            features: Vec::new(),
            series: Vec::new(),
            age_ratings: Vec::new(),
            platforms: Vec::new(),
            game_modes: Vec::new(),
            themes: Vec::new(),
            perspectives: Vec::new(),
            regions: Vec::new(),
            version: None,
            manual: None,
            critic_score: None,
            community_score: None,
            install_size: None,
            time_to_beat: None,
            links: Vec::new(),
            field_sources: BTreeMap::new(),
            completion_status: None,
            user_score: None,
            notes: None,
            categories: vec!["c1".into()],
            copies: vec![GameCopy {
                id,
                source: "steam".into(),
                source_game_id: Some("1145360".into()),
                installed: true,
                playtime_seconds: 60,
                last_played: None,
                qualifier: None,
            }],
            metadata_updated_at: None,
            extra: json!({}),
        }
    }

    fn names() -> HashMap<String, String> {
        HashMap::from([
            ("c1".to_string(), "Roguelikes".to_string()),
            ("c2".to_string(), "VR".to_string()),
        ])
    }

    fn snap(game: &Game) -> EntrySnapshot {
        EntrySnapshot::of(game, &names(), &EntryExtras::default())
    }

    fn fields(changes: &[FieldChange]) -> Vec<&str> {
        changes.iter().map(|c| c.field.as_str()).collect()
    }

    #[test]
    fn plain_values_show_before_and_after() {
        let a = game();
        let mut b = a.clone();
        b.name = "Hades II".into();
        b.release_date = Some("2024-05-06".into());
        b.critic_score = Some(Score {
            value: 90,
            source: "Metacritic".into(),
        });
        b.user_score = Some(80);
        b.favorite = true;
        let changes = diff(&snap(&a), &snap(&b));
        assert_eq!(
            fields(&changes),
            [
                "name",
                "releaseDate",
                "criticScore",
                "userScore",
                "favorite"
            ]
        );
        assert_eq!(changes[0].before, Some(json!("Hades")));
        assert_eq!(changes[0].after, Some(json!("Hades II")));
        // Set from nothing: no `before`.
        assert_eq!(changes[1].before, None);
        assert_eq!(changes[1].mode, ChangeMode::Value);
    }

    #[test]
    fn regions_version_and_manual_show_in_the_diff() {
        let a = game();
        let mut b = a.clone();
        b.regions = vec!["Europe".into()];
        b.version = Some("1.4.2".into());
        b.manual = Some("https://example.org/manual".into());
        let changes = diff(&snap(&a), &snap(&b));
        assert_eq!(fields(&changes), ["version", "manual", "regions"]);
        assert_eq!(changes[0].after, Some(json!("1.4.2")));
        assert_eq!(changes[2].mode, ChangeMode::Set);
    }

    #[test]
    fn lists_show_what_was_added_and_removed() {
        let a = game();
        let mut b = a.clone();
        b.genres = vec!["Action".into(), "Music".into()];
        b.links = vec![GameLink {
            name: "Site".into(),
            url: "https://example.org".into(),
        }];
        let changes = diff(&snap(&a), &snap(&b));
        assert_eq!(fields(&changes), ["genres", "links"]);
        assert_eq!(changes[0].mode, ChangeMode::Set);
        assert_eq!(changes[0].before, Some(json!(["Arcade"])));
        assert_eq!(changes[0].after, Some(json!(["Music"])));
        assert_eq!(
            changes[1].after,
            Some(json!([{ "name": "Site", "url": "https://example.org" }]))
        );
    }

    #[test]
    fn categories_show_names_and_a_rename_is_no_change_of_the_game() {
        let a = game();
        let mut b = a.clone();
        b.categories = vec!["c2".into()];
        let changes = diff(&snap(&a), &snap(&b));
        assert_eq!(changes[0].field, "categories");
        assert_eq!(changes[0].before, Some(json!(["Roguelikes"])));
        assert_eq!(changes[0].after, Some(json!(["VR"])));
        assert_eq!(kind_of(&changes), EntryKind::Categories);

        let renamed = HashMap::from([("c1".to_string(), "Rogue-lites".to_string())]);
        let after = EntrySnapshot::of(&a, &renamed, &EntryExtras::default());
        assert!(diff(&snap(&a), &after).is_empty());
    }

    #[test]
    fn copies_show_their_store_and_count_as_combining() {
        let a = game();
        let mut b = a.clone();
        b.copies.push(GameCopy {
            id: Uuid::new_v4(),
            source: "epic".into(),
            source_game_id: Some("x".into()),
            installed: false,
            playtime_seconds: 0,
            last_played: None,
            qualifier: None,
        });
        let changes = diff(&snap(&a), &snap(&b));
        // With two copies, which one plays first shows too.
        assert_eq!(fields(&changes), ["copies", "preferredCopy"]);
        assert_eq!(
            changes[0].after,
            Some(json!([{ "store": "epic", "storeId": "x", "qualifier": null }]))
        );
        assert_eq!(kind_of(&changes), EntryKind::Combine);
    }

    #[test]
    fn images_show_only_that_they_changed() {
        let a = game();
        let mut b = a.clone();
        b.cover_path = Some("images/covers/b.jpg".into());
        b.icon_path = Some("images/icons/b.png".into());
        let changes = diff(&snap(&a), &snap(&b));
        assert_eq!(fields(&changes), ["cover", "icon"]);
        assert!(changes.iter().all(|c| c.mode == ChangeMode::Changed));
        assert_eq!(changes[1].before, Some(json!(false)));
        assert_eq!(kind_of(&changes), EntryKind::Images);
    }

    #[test]
    fn internal_fields_never_show() {
        let a = game();
        let mut b = a.clone();
        b.sorting_name = "hades ii".into();
        b.playtime_seconds = 9_999;
        b.play_count = 7;
        b.last_played = Some(Utc::now());
        b.metadata_updated_at = Some(Utc::now());
        b.field_sources.insert("genres".into(), "igdb".into());
        b.extra = json!({ "igdbChecked": "2026-01-01", "userRenamed": true });
        let (before, after) = (snap(&a), snap(&b));
        assert!(diff(&before, &after).is_empty());
        assert_ne!(before.playtime, after.playtime);
    }

    #[test]
    fn user_values_from_extra_and_profiles_show() {
        let a = game();
        let mut b = a.clone();
        b.extra = json!({ METADATA_NAME_KEY: "Hades (2020)" });
        let extras = EntryExtras {
            user_sorting_name: Some("Hades 1".into()),
            profiles: vec![ProfileFacts {
                id: "p".into(),
                name: Some("Windowed".into()),
                args: Some("-w".into()),
                target: None,
            }],
            default_profile: Some("p".into()),
        };
        let changes = diff(&snap(&a), &EntrySnapshot::of(&b, &names(), &extras));
        assert_eq!(
            fields(&changes),
            [
                "sortingName",
                "launchProfiles",
                "defaultProfile",
                "metadataName"
            ]
        );
        assert_eq!(changes[2].after, Some(json!("Windowed")));
    }

    #[test]
    fn install_changes_have_their_own_kind() {
        let a = game();
        let mut b = a.clone();
        b.installed = false;
        b.install_dir = None;
        let changes = diff(&snap(&a), &snap(&b));
        assert_eq!(kind_of(&changes), EntryKind::Install);
        b.name = "Other".into();
        assert_eq!(kind_of(&diff(&snap(&a), &snap(&b))), EntryKind::Metadata);
    }

    #[test]
    fn added_and_removed_entries_keep_name_and_stores() {
        let a = game();
        let added = added(&snap(&a));
        assert_eq!(fields(&added), ["name", "copies", "installed"]);
        assert_eq!(added[0].after, Some(json!("Hades")));
        assert_eq!(added[0].before, None);
        let removed = removed(&snap(&a));
        assert_eq!(removed[1].before.as_ref().unwrap()[0]["storeId"], "1145360");
        assert_eq!(removed[1].after, Some(json!([])));
    }
}
