//! Tests of the activity log: storage, recorder, queries, retention, backups, and one test per
//! trigger of the design's coverage table.

use std::path::Path;
use std::sync::{Arc, Mutex};

use chrono::{Duration, Utc};
use serde_json::json;

use super::*;
use crate::db::{Database, NewGameRecord};
use crate::error::CoreError;
use crate::events::EventBus;
use crate::library::{Library, SyncTrigger};
use crate::model::{
    BulkChange, GameEdits, GameMetadata, GameQuery, GameUpdate, ImportedGame, MetadataField,
    NewManualGame, ProfileInput, ScriptInput, ScriptKind, ScriptSpec, TriState, UserEdits,
};
use crate::paths::DataPaths;
use crate::providers::{LibraryProvider, ProviderError, ProviderRegistry};

fn log_with_db() -> (Arc<ActivityLog>, Arc<Database>) {
    let log = Arc::new(ActivityLog::in_memory());
    let db = Arc::new(Database::open_in_memory().unwrap());
    db.attach_activity(log.clone());
    (log, db)
}

fn add_game(db: &Database, name: &str) -> Uuid {
    let _setup = Scope::enter();
    db.insert_game(
        &NewGameRecord {
            name: name.into(),
            source: "manual".into(),
            ..Default::default()
        },
        Utc::now(),
    )
    .unwrap()
    .id
}

fn timeline(log: &ActivityLog, db: &Database, query: ActivityQuery) -> Vec<ActivityItem> {
    log.timeline(db, &query).unwrap().items
}

fn newest(log: &ActivityLog, db: &Database) -> Vec<ActivityItem> {
    timeline(
        log,
        db,
        ActivityQuery {
            limit: 500,
            ..ActivityQuery::default()
        },
    )
}

/// Every entry, newest first, those inside batches too.
fn entries(log: &ActivityLog, db: &Database) -> Vec<ActivityEntry> {
    let query = ActivityQuery {
        include_sessions: false,
        limit: 500,
        ..ActivityQuery::default()
    };
    let mut all = Vec::new();
    for item in timeline(log, db, query) {
        match item {
            ActivityItem::Entry(e) => all.push(e),
            ActivityItem::Batch(b) => {
                for item in log.batch_entries(b.id, None, 500).unwrap().items {
                    if let ActivityItem::Entry(e) = item {
                        all.push(e);
                    }
                }
            }
            ActivityItem::Session(_) => {}
        }
    }
    all
}

fn fields(log: &ActivityLog, entry: &ActivityEntry) -> Vec<String> {
    log.details(entry.id)
        .unwrap()
        .changes
        .into_iter()
        .map(|c| c.field)
        .collect()
}

fn user_edit() -> Change {
    Change::new(Actor::user(), reason::EDIT).kind(EntryKind::Edit)
}

fn favorite(db: &Database, id: Uuid, on: bool) -> crate::error::Result<crate::model::Game> {
    db.update_game(
        id,
        &GameUpdate {
            favorite: Some(on),
            ..GameUpdate::default()
        },
    )
}

// ----- actors -----

#[test]
fn actors_come_from_the_ipc_header() {
    assert_eq!(
        Actor::from_header("plugin:metadata-plus"),
        Some(Actor::plugin("metadata-plus"))
    );
    assert_eq!(Actor::from_header("user"), Some(Actor::user()));
    for bad in [
        "",
        "plugin:",
        "plugin:Bad Id",
        "store:steam",
        "app",
        "plugin:a/b",
    ] {
        assert_eq!(Actor::from_header(bad), None, "{bad}");
    }
}

#[test]
fn the_calling_thread_acts_for_whoever_it_was_told() {
    assert_eq!(current_actor(), Actor::user());
    {
        let _plugin = acting_as(Actor::plugin("x"));
        assert_eq!(Change::by_caller(reason::EDIT).actor, Actor::plugin("x"));
        {
            let _inner = acting_as(Actor::app());
            assert_eq!(current_actor(), Actor::app());
        }
        assert_eq!(current_actor(), Actor::plugin("x"));
    }
    assert_eq!(current_actor(), Actor::user());
}

// ----- storage and recorder -----

#[test]
fn entries_keep_who_why_and_what() {
    let (log, db) = log_with_db();
    let id = add_game(&db, "Hades");
    let change = user_edit().args(json!({ "source": "igdb" }));
    db.record(&change, &[id], || {
        db.update_game(
            id,
            &GameUpdate {
                name: Some("Hades II".into()),
                favorite: Some(true),
                ..GameUpdate::default()
            },
        )
    })
    .unwrap();
    let list = entries(&log, &db);
    assert_eq!(list.len(), 1);
    let entry = &list[0];
    assert_eq!(entry.actor, Actor::user());
    assert_eq!(entry.reason, "edit");
    assert_eq!(entry.args, json!({ "source": "igdb" }));
    assert_eq!(entry.game_id, Some(id));
    assert_eq!(entry.game_name, "Hades II");
    assert_eq!(entry.kind, EntryKind::Edit);
    assert_eq!(entry.change_count, 2);
    let details = log.details(entry.id).unwrap();
    assert_eq!(details.changes[0].field, "name");
    assert_eq!(details.changes[0].before, Some(json!("Hades")));
    assert_eq!(details.changes[0].after, Some(json!("Hades II")));
    assert_eq!(details.changes[1].field, "favorite");
    assert!(details.batch.is_none());
    assert!(log.details(entry.id + 1000).is_err());
}

#[test]
fn a_change_without_visible_effect_writes_nothing() {
    let (log, db) = log_with_db();
    let id = add_game(&db, "Hades");
    db.record(&user_edit(), &[id], || {
        favorite(&db, id, false)?;
        db.set_extra_value(id, "igdbCheckedAt", Some(json!("now")))
    })
    .unwrap();
    assert!(entries(&log, &db).is_empty());
}

#[test]
fn a_failed_change_writes_nothing_and_keeps_its_error() {
    let (log, db) = log_with_db();
    let id = add_game(&db, "Hades");
    let err = db
        .record(&user_edit(), &[id], || -> crate::error::Result<()> {
            favorite(&db, id, true)?;
            Err(CoreError::Invalid("refused".into()))
        })
        .unwrap_err();
    assert!(err.to_string().contains("refused"));
    assert!(entries(&log, &db).is_empty());
}

#[test]
fn a_broken_log_never_fails_the_change() {
    let (log, db) = log_with_db();
    let id = add_game(&db, "Hades");
    log.conn().execute_batch("DROP TABLE changes;").unwrap();
    let game = db
        .record(&user_edit(), &[id], || favorite(&db, id, true))
        .unwrap();
    assert!(game.favorite);
}

#[test]
#[should_panic(expected = "outside a recorded change")]
fn an_unrecorded_write_fails_in_tests() {
    let (_log, db) = log_with_db();
    let id = add_game(&db, "Hades");
    favorite(&db, id, true).unwrap();
}

#[test]
fn internal_writes_and_databases_without_a_log_are_not_checked() {
    let (_log, db) = log_with_db();
    let id = add_game(&db, "Hades");
    db.set_state_version(id, 3).unwrap();
    db.mark_metadata_checked(id, Utc::now()).unwrap();
    db.set_extra_value(id, "x", Some(json!(1))).unwrap();
    let session = db.begin_session(id, Utc::now()).unwrap();
    db.heartbeat_session(session, Utc::now()).unwrap();
    db.finish_session(session, Utc::now(), 60).unwrap();
    db.recompute_sorting_names(&[]).unwrap();
    let plain = Database::open_in_memory().unwrap();
    let other = plain
        .insert_game(
            &NewGameRecord {
                name: "Celeste".into(),
                source: "manual".into(),
                ..Default::default()
            },
            Utc::now(),
        )
        .unwrap();
    favorite(&plain, other.id, true).unwrap();
}

#[test]
fn batches_count_their_work_and_vanish_when_empty() {
    let (log, db) = log_with_db();
    let a = add_game(&db, "A");
    let b = add_game(&db, "B");
    let change = Change::new(Actor::user(), reason::BULK_CHANGE);
    db.record_group(&change, &[a, b], || {
        db.bulk_change(
            &[a, b],
            &BulkChange {
                favorite: Some(true),
                ..BulkChange::default()
            },
        )
    })
    .unwrap();
    let items = newest(&log, &db);
    let [ActivityItem::Batch(batch)] = items.as_slice() else {
        panic!("one batch: {items:?}");
    };
    assert_eq!((batch.entry_count, batch.game_count), (2, 2));
    assert!(batch.finished_at.is_some());
    assert_eq!(log.batch(batch.id).unwrap().as_ref(), Some(batch));
    // A game's log shows its part of the batch, and its details name the batch.
    let own = timeline(
        &log,
        &db,
        ActivityQuery {
            game_id: Some(a),
            ..ActivityQuery::default()
        },
    );
    let [ActivityItem::Entry(part)] = own.as_slice() else {
        panic!("one entry: {own:?}");
    };
    assert_eq!(part.batch_id, Some(batch.id));
    assert_eq!(
        log.details(part.id).unwrap().batch.map(|b| b.id),
        Some(batch.id)
    );
    // The batch's entries page by page.
    let first = log.batch_entries(batch.id, None, 1).unwrap();
    assert_eq!(first.items.len(), 1);
    let rest = log.batch_entries(batch.id, first.next, 1).unwrap();
    assert_eq!(rest.items.len(), 1);
    assert!(rest.next.is_none());

    // Nothing changed: no batch. One entry: no batch either.
    db.begin_batch(&change).finish();
    let single = db.begin_batch(&change);
    db.record(single.change(), &[a], || favorite(&db, a, false))
        .unwrap();
    single.finish();
    let items = newest(&log, &db);
    assert_eq!(items.len(), 2, "{items:?}");
    assert!(matches!(&items[0], ActivityItem::Entry(e) if e.batch_id.is_none()));

    // Counts alone keep a batch; zero counts are left out.
    let mut counted = db.begin_batch(&Change::new(Actor::store("steam"), reason::SYNC));
    counted.count("playtime", 3);
    counted.count("added", 0);
    counted.finish();
    let items = newest(&log, &db);
    let ActivityItem::Batch(b) = &items[0] else {
        panic!("a batch first: {items:?}");
    };
    assert_eq!(b.actor, Actor::store("steam"));
    assert_eq!(
        b.counts,
        std::collections::BTreeMap::from([("playtime".to_string(), 3)])
    );
}

#[test]
fn library_level_entries_have_no_game() {
    let (log, db) = log_with_db();
    log.note(
        &Change::new(Actor::user(), reason::CATEGORY_CREATE).args(json!({ "name": "VR" })),
        EntryKind::Categories,
        Vec::new(),
    );
    let list = entries(&log, &db);
    assert_eq!(list[0].game_id, None);
    assert_eq!(list[0].game_name, "");
    assert_eq!(list[0].args["name"], "VR");
}

#[test]
fn the_timeline_merges_sessions_and_pages_without_gaps() {
    let (log, db) = log_with_db();
    let hades = add_game(&db, "Hades");
    let celeste = add_game(&db, "Celeste");
    {
        let _setup = Scope::enter();
        for days in 1..=5 {
            db.add_session(hades, Utc::now() - Duration::days(days), 600)
                .unwrap();
        }
        db.add_session(celeste, Utc::now() - Duration::hours(3), 60)
            .unwrap();
    }
    for i in 0..7 {
        db.record(&user_edit(), &[hades], || favorite(&db, hades, i % 2 == 0))
            .unwrap();
    }
    // Every item once, newest first, over pages of 3.
    let mut seen = Vec::new();
    let mut before = None;
    loop {
        let page = log
            .timeline(
                &db,
                &ActivityQuery {
                    before,
                    limit: 3,
                    ..ActivityQuery::default()
                },
            )
            .unwrap();
        assert!(page.items.len() <= 3);
        seen.extend(page.items.iter().map(ActivityItem::cursor));
        match page.next {
            Some(next) => before = Some(next),
            None => break,
        }
    }
    assert_eq!(seen.len(), 13);
    let mut sorted = seen.clone();
    sorted.sort_by(|a, b| b.cmp(a));
    sorted.dedup();
    assert_eq!(seen, sorted, "newest first, no repeats");
    let all = newest(&log, &db);
    assert!(
        matches!(&all[7], ActivityItem::Session(s) if s.game_name == "Celeste" && s.duration_seconds == 60 && s.game_id == celeste),
        "{:?}",
        all[7]
    );

    // One game's log; without sessions; a plugin's log has no sessions.
    let game = timeline(
        &log,
        &db,
        ActivityQuery {
            game_id: Some(celeste),
            ..ActivityQuery::default()
        },
    );
    assert_eq!(game.len(), 1);
    let no_sessions = ActivityQuery {
        include_sessions: false,
        limit: 100,
        ..ActivityQuery::default()
    };
    assert_eq!(timeline(&log, &db, no_sessions).len(), 7);
    let plugin = ActivityQuery {
        actors: vec![ActorFilter {
            kind: ActorKind::Plugin,
            id: None,
        }],
        ..ActivityQuery::default()
    };
    assert!(timeline(&log, &db, plugin).is_empty());
    // Search, time and kind filters.
    let searched = ActivityQuery {
        search: Some("cel".into()),
        ..ActivityQuery::default()
    };
    assert_eq!(timeline(&log, &db, searched).len(), 1);
    let today = ActivityQuery {
        from: Some(Utc::now() - Duration::hours(1)),
        limit: 100,
        ..ActivityQuery::default()
    };
    assert_eq!(timeline(&log, &db, today).len(), 7);
    let removed = ActivityQuery {
        kinds: vec![EntryKind::Removed],
        include_sessions: false,
        ..ActivityQuery::default()
    };
    assert!(timeline(&log, &db, removed).is_empty());
    let edits = ActivityQuery {
        kinds: vec![EntryKind::Edit],
        include_sessions: false,
        ..ActivityQuery::default()
    };
    assert_eq!(timeline(&log, &db, edits).len(), 7);
}

fn event(source: &str, level: SystemLevel, kind: &str, message: &str) -> NewSystemEvent {
    let (source_kind, source_id) = NewSystemEvent::source(source);
    NewSystemEvent {
        level,
        source_kind,
        source_id,
        kind: kind.into(),
        message: message.into(),
        details: Some(json!({ "host": "api.example.org" })),
    }
}

#[test]
fn system_events_are_filtered_and_plugin_requests_can_be_off() {
    let log = ActivityLog::in_memory();
    log.system_event(event(
        "plugin:meta",
        SystemLevel::Info,
        "http",
        "37 requests",
    ));
    log.system_event(event("plugin:meta", SystemLevel::Error, "log", "boom"));
    log.system_event(event(
        "store:steam",
        SystemLevel::Warn,
        "signInExpired",
        "expired",
    ));
    log.system_event(event("app", SystemLevel::Info, "backup", "backup written"));
    let read = |query: SystemQuery| log.system_events(&query).unwrap().items;
    let all = read(SystemQuery::default());
    assert_eq!(all.len(), 4);
    assert_eq!(all[0].source_kind, "app");
    assert_eq!(all[1].source_id.as_deref(), Some("steam"));
    let plugins = SystemQuery {
        sources: vec!["plugin".into()],
        ..SystemQuery::default()
    };
    assert_eq!(read(plugins).len(), 2);
    let warn = read(SystemQuery {
        min_level: Some(SystemLevel::Warn),
        ..SystemQuery::default()
    });
    assert_eq!(warn.len(), 2);
    assert_eq!(warn[0].message, "expired");
    let http = read(SystemQuery {
        sources: vec!["plugin:meta".into()],
        kinds: vec!["http".into()],
        search: Some("37".into()),
        ..SystemQuery::default()
    });
    assert_eq!(http.len(), 1);
    assert_eq!(http[0].details["host"], "api.example.org");

    log.set_settings(ActivitySettings {
        record_plugin_requests: false,
        ..ActivitySettings::default()
    });
    log.system_event(event("plugin:meta", SystemLevel::Info, "http", "more"));
    assert_eq!(read(SystemQuery::default()).len(), 4, "requests not kept");

    let page = log
        .system_events(&SystemQuery {
            limit: 3,
            ..SystemQuery::default()
        })
        .unwrap();
    assert_eq!(page.items.len(), 3);
    let rest = log
        .system_events(&SystemQuery {
            before: page.next,
            ..SystemQuery::default()
        })
        .unwrap();
    assert_eq!(rest.items.len(), 1);
    assert!(rest.next.is_none());

    log.clear(ClearPart::System).unwrap();
    assert!(read(SystemQuery::default()).is_empty());
}

#[test]
fn clearing_one_part_keeps_the_other() {
    let (log, db) = log_with_db();
    let id = add_game(&db, "Hades");
    db.record(&user_edit(), &[id], || favorite(&db, id, true))
        .unwrap();
    log.system_event(event("app", SystemLevel::Info, "backup", "done"));
    log.clear(ClearPart::Library).unwrap();
    let stats = log.stats().unwrap();
    assert_eq!((stats.entries, stats.system_events), (0, 1));
    log.clear(ClearPart::All).unwrap();
    assert_eq!(log.stats().unwrap().system_events, 0);
}

#[test]
fn retention_drops_old_system_events_and_the_oldest_entries_over_the_size_limit() {
    let tmp = tempfile::tempdir().unwrap();
    let log = ActivityLog::open(&tmp.path().join(FILE_NAME)).unwrap();
    log.set_settings(ActivitySettings {
        library_max_mb: 1,
        system_days: 30,
        record_plugin_requests: true,
    });
    let day = 24 * 60 * 60 * 1000;
    log.conn()
        .execute(
            "INSERT INTO system_events (at, level, source_kind, kind, message) \
             VALUES (?1, 'info', 'app', 'old', 'old'), (?2, 'info', 'app', 'new', 'new')",
            [now_ms() - 31 * day, now_ms() - 29 * day],
        )
        .unwrap();
    // About 3 MB of descriptions.
    let text = "x".repeat(10_000);
    for i in 0..300 {
        log.write(
            &Change::new(Actor::app(), reason::METADATA_FETCH),
            vec![NewEntry {
                game_id: Some(Uuid::new_v4()),
                game_name: format!("Game {i}"),
                kind: EntryKind::Metadata,
                changes: vec![FieldChange::value(
                    "description",
                    None,
                    Some(json!(format!("{i} {text}"))),
                )],
                args: None,
            }],
        );
    }
    let before = log.stats().unwrap();
    assert!(before.library_bytes > 2 * 1024 * 1024, "{before:?}");
    log.prune().unwrap();
    let after = log.stats().unwrap();
    assert!(after.library_bytes <= 1024 * 1024, "{after:?}");
    assert!(after.entries > 0 && after.entries < 300, "{after:?}");
    assert_eq!(after.system_events, 1, "the month-old event went");
    // The newest entries stay.
    let newest: String = log
        .conn()
        .query_row(
            "SELECT game_name FROM entries ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(newest, "Game 299");
}

#[test]
fn a_broken_log_file_is_set_aside() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join(FILE_NAME);
    std::fs::write(&path, b"not a database at all, not even close to one").unwrap();
    let log = ActivityLog::open_or_fresh(&path);
    assert_eq!(log.stats().unwrap().entries, 0);
    assert!(tmp.path().join("activity.sqlite.broken").is_file());
}

// ----- the library: one test per trigger of the coverage table -----

struct Store {
    id: &'static str,
    plugin: Option<&'static str>,
    games: Mutex<Vec<ImportedGame>>,
}

impl Store {
    fn new(id: &'static str, games: Vec<ImportedGame>) -> Arc<Self> {
        Arc::new(Self {
            id,
            plugin: None,
            games: Mutex::new(games),
        })
    }

    fn set(&self, games: Vec<ImportedGame>) {
        *self.games.lock().unwrap() = games;
    }
}

impl LibraryProvider for Store {
    fn id(&self) -> &str {
        self.id
    }
    fn name(&self) -> &str {
        "Store"
    }
    fn icon_svg(&self) -> &'static str {
        "<svg/>"
    }
    fn plugin_id(&self) -> Option<&str> {
        self.plugin
    }
    fn is_installed(&self) -> bool {
        true
    }
    fn scan_installed(&self) -> Result<Vec<ImportedGame>, ProviderError> {
        Ok(self.games.lock().unwrap().clone())
    }
}

fn library_with(paths: DataPaths, stores: Vec<Arc<Store>>, db: Arc<Database>) -> Library {
    paths.ensure_dirs().unwrap();
    let mut providers = ProviderRegistry::new();
    for store in stores {
        providers.register(store);
    }
    Library::with_database(
        paths,
        db,
        providers,
        EventBus::new(),
        Arc::new(crate::auth::MemoryTokenStore::default()),
    )
}

fn library(tmp: &Path, stores: Vec<Arc<Store>>) -> Library {
    library_with(
        DataPaths::new(tmp.join("data")),
        stores,
        Arc::new(Database::open_in_memory().unwrap()),
    )
}

fn lib_entries(lib: &Library) -> Vec<ActivityEntry> {
    entries(lib.activity(), lib.database())
}

fn manual_game(lib: &Library, tmp: &Path, name: &str) -> Uuid {
    let exe = tmp.join(format!("{name}.exe"));
    std::fs::write(&exe, b"").unwrap();
    lib.add_manual_game(&NewManualGame {
        name: Some(name.into()),
        executable: exe.display().to_string(),
        args: None,
        working_dir: None,
        cover_image: None,
    })
    .unwrap()
    .id
}

#[test]
fn user_commands_record_entries_by_the_caller() {
    let tmp = tempfile::tempdir().unwrap();
    let lib = library(tmp.path(), Vec::new());
    let id = manual_game(&lib, tmp.path(), "Hades");
    let added = lib_entries(&lib);
    assert_eq!(added.len(), 1);
    assert_eq!(added[0].kind, EntryKind::Added);
    assert_eq!(added[0].reason, "addManual");
    assert_eq!(added[0].actor, Actor::user());

    {
        let _plugin = acting_as(Actor::plugin("tidy"));
        lib.update_game(
            id,
            &GameUpdate {
                hidden: Some(true),
                ..GameUpdate::default()
            },
        )
        .unwrap();
    }
    let category = lib.create_category("Roguelikes").unwrap();
    lib.set_game_categories(id, std::slice::from_ref(&category.id))
        .unwrap();
    lib.rename_category(&category.id, "Rogue-lites").unwrap();
    lib.add_game_session(id, Utc::now() - Duration::days(1), 3600)
        .unwrap();
    lib.remove_game(id).unwrap();

    let list = lib_entries(&lib);
    let reasons: Vec<&str> = list.iter().rev().map(|e| e.reason.as_str()).collect();
    assert_eq!(
        reasons,
        [
            "addManual",
            "edit",
            "categoryCreate",
            "setCategories",
            "categoryRename",
            "sessionEdit",
            "remove"
        ]
    );
    let hidden = list.iter().find(|e| e.reason == "edit").unwrap();
    assert_eq!(hidden.actor, Actor::plugin("tidy"), "the plugin did it");
    assert_eq!(hidden.kind, EntryKind::Edit);
    assert_eq!(fields(lib.activity(), hidden), ["hidden"]);
    let set = list.iter().find(|e| e.reason == "setCategories").unwrap();
    assert_eq!(set.kind, EntryKind::Categories);
    let details = lib.activity().details(set.id).unwrap();
    assert_eq!(details.changes[0].field, "categories");
    assert_eq!(details.changes[0].after, Some(json!(["Roguelikes"])));
    let session = list.iter().find(|e| e.reason == "sessionEdit").unwrap();
    assert_eq!(session.kind, EntryKind::Sessions);
    let removed = &list[0];
    assert_eq!(removed.kind, EntryKind::Removed);
    assert_eq!(removed.game_name, "Hades");
    // The removed game's history still reads.
    let own = lib
        .activity()
        .timeline(
            lib.database(),
            &ActivityQuery {
                game_id: Some(id),
                limit: 100,
                ..ActivityQuery::default()
            },
        )
        .unwrap();
    assert_eq!(own.items.len(), 5, "{:?}", own.items);
}

#[test]
fn edits_images_profiles_and_combining_are_recorded() {
    let tmp = tempfile::tempdir().unwrap();
    let lib = library(tmp.path(), Vec::new());
    let a = manual_game(&lib, tmp.path(), "Doom");
    let b = manual_game(&lib, tmp.path(), "Doom 2016");
    let mut edits = UserEdits::default();
    edits.meta.description = Some("Rip and tear.".into());
    edits.edited = vec![MetadataField::Description];
    lib.save_game_edits(
        a,
        &GameEdits {
            edits,
            user_score: Some(90),
            ..GameEdits::default()
        },
    )
    .unwrap();
    let cover = tmp.path().join("cover.png");
    std::fs::write(&cover, b"png").unwrap();
    lib.set_cover(a, Some(&cover)).unwrap();
    lib.save_launch_profile(
        a,
        Some(a),
        &ProfileInput {
            args: Some("-windowed".into()),
            ..ProfileInput::default()
        },
    )
    .unwrap();
    lib.combine_games(a, &[b]).unwrap();

    let log = lib.activity();
    let list = lib_entries(&lib);
    let by = |reason: &str| -> Vec<&ActivityEntry> {
        list.iter().filter(|e| e.reason == reason).collect()
    };
    let edit = by("edit");
    assert!(
        edit.iter()
            .any(|e| e.kind == EntryKind::Edit && fields(log, e) == ["description", "userScore"]),
        "{edit:?}"
    );
    assert!(
        edit.iter()
            .any(|e| e.kind == EntryKind::Images && fields(log, e) == ["cover"]),
        "{edit:?}"
    );
    let profiles = by("launchProfiles");
    assert_eq!(profiles.len(), 1, "{list:?}");
    assert_eq!(profiles[0].kind, EntryKind::Profiles);
    let combine = by("combine");
    assert_eq!(combine.len(), 2, "the entry and the one combined into it");
    assert!(combine.iter().all(|e| e.kind == EntryKind::Combine));
    let absorbed = combine.iter().find(|e| e.game_id == Some(b)).unwrap();
    let details = log.details(absorbed.id).unwrap();
    assert_eq!(details.changes[0].field, "combinedInto");
    // The combined entry's log includes the absorbed game's history.
    let own = log
        .timeline(
            lib.database(),
            &ActivityQuery {
                game_id: Some(a),
                limit: 100,
                ..ActivityQuery::default()
            },
        )
        .unwrap();
    assert!(own.items.iter().any(
        |i| matches!(i, ActivityItem::Entry(e) if e.game_id == Some(b) && e.reason == "addManual")
    ));
}

#[test]
fn a_sync_is_one_batch_by_its_store_with_counts() {
    let tmp = tempfile::tempdir().unwrap();
    let steam = Store::new(
        "steam",
        vec![
            ImportedGame::installed("1", "Hades"),
            ImportedGame::installed("2", "Celeste"),
        ],
    );
    let lib = library(tmp.path(), vec![steam.clone()]);
    lib.scan("steam").unwrap();
    let items = newest(lib.activity(), lib.database());
    let [ActivityItem::Batch(first)] = items.as_slice() else {
        panic!("one batch: {items:?}");
    };
    assert_eq!(first.actor, Actor::store("steam"));
    assert_eq!(first.reason, "sync");
    assert_eq!(first.args["store"], "steam");
    assert_eq!(first.args["trigger"], "manual");
    assert_eq!(first.counts.get("added"), Some(&2));
    assert_eq!(first.entry_count, 2);

    // Renamed, uninstalled, playtime from the store.
    let mut hades = ImportedGame::installed("1", "Hades (2020)");
    hades.playtime_seconds = Some(3600);
    let mut celeste = ImportedGame::installed("2", "Celeste");
    celeste.installed = false;
    steam.set(vec![hades, celeste]);
    lib.scan_as("steam", true).unwrap();
    let items = newest(lib.activity(), lib.database());
    let ActivityItem::Batch(second) = &items[0] else {
        panic!("a batch: {items:?}");
    };
    assert_eq!(second.reason, "autoSync");
    assert_eq!(second.args["trigger"], "auto");
    assert_eq!(second.counts.get("uninstalled"), Some(&1), "{second:?}");
    assert_eq!(second.counts.get("playtime"), Some(&1), "{second:?}");
    let parts: Vec<(String, EntryKind)> = lib
        .activity()
        .batch_entries(second.id, None, 10)
        .unwrap()
        .items
        .iter()
        .map(|i| match i {
            ActivityItem::Entry(e) => (e.game_name.clone(), e.kind),
            other => panic!("{other:?}"),
        })
        .collect();
    assert!(
        parts.contains(&("Hades (2020)".into(), EntryKind::Metadata)),
        "{parts:?}"
    );
    assert!(
        parts.contains(&("Celeste".into(), EntryKind::Install)),
        "{parts:?}"
    );

    // A sync that changes nothing leaves nothing.
    lib.scan("steam").unwrap();
    assert_eq!(newest(lib.activity(), lib.database()).len(), 2);
}

#[test]
fn a_plugins_store_is_synced_by_the_plugin() {
    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(Store {
        id: "bsmanager",
        plugin: Some("bsmanager"),
        games: Mutex::new(vec![ImportedGame::installed("1.40.0", "Beat Saber")]),
    });
    let lib = library(tmp.path(), vec![store]);
    lib.sync("bsmanager", SyncTrigger::Startup).unwrap();
    let items = newest(lib.activity(), lib.database());
    let ActivityItem::Batch(batch) = &items[0] else {
        panic!("{items:?}");
    };
    assert_eq!(batch.actor, Actor::plugin("bsmanager"));
    assert_eq!(batch.args["trigger"], "startup");
    let filtered = timeline(
        lib.activity(),
        lib.database(),
        ActivityQuery {
            actors: vec![ActorFilter {
                kind: ActorKind::Plugin,
                id: Some("bsmanager".into()),
            }],
            ..ActivityQuery::default()
        },
    );
    assert_eq!(filtered.len(), 1);
}

struct MetaStore;

impl LibraryProvider for MetaStore {
    fn id(&self) -> &str {
        "meta"
    }
    fn name(&self) -> &str {
        "Meta"
    }
    fn icon_svg(&self) -> &'static str {
        "<svg/>"
    }
    fn scan_installed(&self) -> Result<Vec<ImportedGame>, ProviderError> {
        Ok(Vec::new())
    }
    fn metadata(&self, _id: &str) -> Result<Option<GameMetadata>, ProviderError> {
        Ok(Some(GameMetadata {
            description: Some("From the store.".into()),
            genres: vec!["Action".into()],
            ..GameMetadata::default()
        }))
    }
}

#[test]
fn background_fetchers_record_per_game_in_one_batch_per_run() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = DataPaths::new(tmp.path().join("data"));
    paths.ensure_dirs().unwrap();
    let (log, db) = log_with_db();
    let ids: Vec<Uuid> = ["a", "b"]
        .iter()
        .map(|id| {
            let _setup = Scope::enter();
            db.insert_game(
                &NewGameRecord {
                    name: format!("Game {id}"),
                    source: "meta".into(),
                    source_game_id: Some((*id).into()),
                    ..Default::default()
                },
                Utc::now(),
            )
            .unwrap()
            .id
        })
        .collect();
    let mut providers = ProviderRegistry::new();
    providers.register(Arc::new(MetaStore));
    let fetcher = crate::images::MetadataFetcher::new(
        db.clone(),
        crate::images::ImageCache::new(paths),
        providers,
        EventBus::new(),
        crate::images::SteamGridDb::new(None),
        crate::images::Igdb::new(None, None),
        crate::metadata::MetadataResolver::new(
            db.clone(),
            &ProviderRegistry::new(),
            Default::default(),
        ),
        std::time::Duration::ZERO,
    );
    fetcher.enqueue(ids.iter().map(|id| crate::images::MetadataJob {
        game_id: *id,
        source: "meta".into(),
        source_game_id: Some("x".into()),
        igdb: false,
        force: false,
        by: Actor::app(),
    }));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let batch = loop {
        let items = newest(&log, &db);
        if let Some(ActivityItem::Batch(b)) = items.first()
            && b.finished_at.is_some()
        {
            break b.clone();
        }
        assert!(
            std::time::Instant::now() < deadline,
            "no finished batch: {items:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    fetcher.close();
    assert_eq!(batch.actor, Actor::app());
    assert_eq!(batch.reason, "metadataFetch");
    assert_eq!(batch.entry_count, 2);
    let list = entries(&log, &db);
    assert!(list.iter().all(|e| e.kind == EntryKind::Metadata));
    assert_eq!(fields(&log, &list[0]), ["description", "genres"]);
}

#[test]
fn app_work_is_recorded_as_questkeep() {
    let tmp = tempfile::tempdir().unwrap();
    let lib = library(tmp.path(), Vec::new());
    let id = manual_game(&lib, tmp.path(), "Hades");
    // A category filter given to matching games: a batch with the library-level entry.
    let category = lib.create_category("Everything").unwrap();
    lib.set_category_filter(&category.id, Some(&GameQuery::default()))
        .unwrap();
    let list = lib_entries(&lib);
    assert!(
        list.iter()
            .any(|e| e.reason == "categoryFilter" && e.game_id == Some(id)),
        "{list:?}"
    );

    // The first start moves a new game on, by Questkeep.
    let mut settings = lib.completion_settings().unwrap();
    let statuses: Vec<String> = settings.statuses.iter().map(|s| s.id.clone()).collect();
    settings.new_game = Some(statuses[0].clone());
    settings.first_start = Some(statuses[1].clone());
    lib.set_completion_settings(&settings, &Default::default())
        .unwrap();
    lib.set_completion_status(&[id], Some(&statuses[0]))
        .unwrap();
    lib.database().begin_session(id, Utc::now()).unwrap();
    let list = lib_entries(&lib);
    let first = list
        .iter()
        .find(|e| e.reason == "firstStart")
        .expect("first start recorded");
    assert_eq!(first.actor, Actor::app());
    assert_eq!(first.kind, EntryKind::Status);
    assert_eq!(fields(lib.activity(), first), ["completionStatus"]);

    // Computing all metadata again changes nothing visible: nothing written.
    let before = lib_entries(&lib).len();
    lib.database()
        .record_batch(&Change::new(Actor::app(), reason::RESOLVE_ALL), None, || {
            lib.resolver().resolve_all()
        })
        .unwrap();
    assert_eq!(lib_entries(&lib).len(), before);
}

#[test]
fn the_auto_category_worker_records_as_questkeep() {
    let tmp = tempfile::tempdir().unwrap();
    let lib = library(tmp.path(), Vec::new());
    let category = lib.create_category("Favorites").unwrap();
    lib.set_category_filter(
        &category.id,
        Some(&GameQuery {
            favorite: TriState::Yes,
            ..GameQuery::default()
        }),
    )
    .unwrap();
    let id = manual_game(&lib, tmp.path(), "Hades");
    lib.update_game(
        id,
        &GameUpdate {
            favorite: Some(true),
            ..GameUpdate::default()
        },
    )
    .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let auto = lib_entries(&lib)
            .into_iter()
            .find(|e| e.reason == "autoCategories");
        if let Some(entry) = auto {
            assert_eq!(entry.actor, Actor::app());
            assert_eq!(entry.kind, EntryKind::Categories);
            assert_eq!(entry.game_id, Some(id));
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "no auto category entry"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
fn scripts_are_recorded() {
    let tmp = tempfile::tempdir().unwrap();
    let lib = library(tmp.path(), Vec::new());
    let id = manual_game(&lib, tmp.path(), "Hades");
    lib.scripts()
        .save(&ScriptInput {
            id: None,
            name: "Mods".into(),
            kind: ScriptKind::Pre,
            enabled: true,
            game_id: Some(id),
            filter: None,
            spec: ScriptSpec {
                program: "C:/mods.exe".into(),
                ..ScriptSpec::default()
            },
        })
        .unwrap();
    let list = lib_entries(&lib);
    assert_eq!(list[0].reason, "scriptSave", "{list:?}");
    assert_eq!(list[0].kind, EntryKind::Scripts);
    assert_eq!(list[0].game_id, Some(id));
}

/// The staged restore in a data folder of its own: the library's worker threads keep the
/// current files open, which Windows does not let the restore move (the app restores before it
/// opens them).
fn staged_restore(paths: &DataPaths, tmp: &Path) -> DataPaths {
    fn copy(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for item in std::fs::read_dir(from).unwrap() {
            let item = item.unwrap();
            let target = to.join(item.file_name());
            if item.file_type().unwrap().is_dir() {
                copy(&item.path(), &target);
            } else {
                std::fs::copy(item.path(), target).unwrap();
            }
        }
    }
    let staged = DataPaths::new(tmp.join("restored"));
    staged.ensure_dirs().unwrap();
    copy(
        &paths.root().join("restore-pending"),
        &staged.root().join("restore-pending"),
    );
    staged
}

#[test]
fn backups_carry_the_log_and_a_restore_is_noted() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = DataPaths::new(tmp.path().join("data"));
    let lib = library_with(
        paths.clone(),
        Vec::new(),
        Arc::new(Database::open_in_memory().unwrap()),
    );
    manual_game(&lib, tmp.path(), "Hades");
    let backup = lib.run_backup().unwrap();
    manual_game(&lib, tmp.path(), "Celeste");
    assert_eq!(lib_entries(&lib).len(), 2);
    let names: Vec<String> = zip::ZipArchive::new(std::fs::File::open(&backup.path).unwrap())
        .unwrap()
        .file_names()
        .filter_map(|n| n.ok().map(|n| n.to_string()))
        .collect();
    assert!(names.iter().any(|n| n == FILE_NAME), "{names:?}");
    lib.import_backup(Path::new(&backup.path), false).unwrap();
    let paths = staged_restore(&paths, tmp.path());
    drop(lib);
    assert!(crate::backup::apply_pending_restore(&paths).unwrap());
    assert!(paths.restored_marker().is_file());

    // The restored log (Hades only) plus the restore itself.
    let db = Arc::new(Database::open(&paths.database_file()).unwrap());
    let reopened = library_with(paths.clone(), Vec::new(), db);
    let list = lib_entries(&reopened);
    let reasons: Vec<&str> = list.iter().map(|e| e.reason.as_str()).collect();
    assert_eq!(reasons, ["restoreBackup", "addManual"]);
    assert_eq!(list[1].game_name, "Hades");
    assert!(list[0].args["createdAt"].is_string());
    assert!(!paths.restored_marker().exists(), "noted once");
}

#[test]
fn an_old_backup_without_a_log_restores_with_an_empty_one() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = DataPaths::new(tmp.path().join("data"));
    let lib = library_with(
        paths.clone(),
        Vec::new(),
        Arc::new(Database::open_in_memory().unwrap()),
    );
    manual_game(&lib, tmp.path(), "Hades");
    let never = std::sync::atomic::AtomicBool::new(false);
    let old = crate::backup::create_backup(
        lib.database(),
        None,
        &paths,
        &tmp.path().join("backups"),
        5,
        &never,
        &|_, _| {},
    )
    .unwrap();
    lib.import_backup(Path::new(&old.path), false).unwrap();
    let paths = staged_restore(&paths, tmp.path());
    drop(lib);
    // The current log is set aside, not kept.
    std::fs::write(paths.activity_file(), b"the old log").unwrap();
    assert!(crate::backup::apply_pending_restore(&paths).unwrap());
    let log = ActivityLog::open(&paths.activity_file()).unwrap();
    assert_eq!(log.stats().unwrap().entries, 0);
}

/// The design's performance goal: a sync of 2 000 games stays fast (one bulk snapshot before
/// and after, one transaction for its entries).
#[test]
#[allow(clippy::print_stderr)]
fn a_sync_of_2000_games_stays_fast() {
    let tmp = tempfile::tempdir().unwrap();
    let games: Vec<ImportedGame> = (0..2000)
        .map(|i| ImportedGame::installed(i.to_string(), format!("Game {i}")))
        .collect();
    let steam = Store::new("steam", games.clone());
    let lib = library(tmp.path(), vec![steam.clone()]);
    let timed = |label: &str| {
        let started = std::time::Instant::now();
        lib.scan("steam").unwrap();
        let elapsed = started.elapsed();
        eprintln!("{label}: {elapsed:?}");
        elapsed
    };
    let first = timed("first sync, 2000 added");
    steam.set(
        games
            .iter()
            .map(|g| {
                ImportedGame::installed(g.source_game_id.clone(), format!("{} (2026)", g.name))
            })
            .collect(),
    );
    let second = timed("second sync, 2000 renamed");
    let third = timed("third sync, nothing changed");
    let started = std::time::Instant::now();
    lib.database().snapshot(None).unwrap();
    eprintln!("one snapshot of 2000 games: {:?}", started.elapsed());
    let stats = lib.activity().stats().unwrap();
    eprintln!(
        "entries: {}, library part: {} bytes",
        stats.entries, stats.library_bytes
    );
    assert_eq!(stats.entries, 4000);
    // Generous bounds: debug builds on slow CI machines.
    for elapsed in [first, second, third] {
        assert!(elapsed < std::time::Duration::from_secs(30), "{elapsed:?}");
    }
}
