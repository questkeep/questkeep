//! The activity log: who changed what in the library and why, plus what the app and its plugins
//! did behind the scenes. Kept in its own database, `<data>/activity.sqlite`, so the library
//! database stays small and retention is a matter of this file's size.
//!
//! - **Entries** are written by the recorder ([`crate::db::Database::record`] and friends): the
//!   write sites in the core wrap their mutation, the recorder snapshots the affected entries
//!   before and after and stores what [`diff`] finds, one entry per game that visibly changed.
//! - **Batches** group the entries of one sync, bulk edit or fetch run.
//! - **System events** come from `tracing` events with the target `activity` (the shell installs
//!   the layer that hands them to [`ActivityLog::system_event`]).
//! - **Sessions** are not copied: the timeline reads them from the library database.
//!
//! In debug builds the game-mutating `Database` functions check that a recording scope is active
//! ([`in_scope`]); unit tests fail on a forgotten write site.

pub mod diff;
mod migrations;
pub mod query;
mod recorder;

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError, RwLock};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;
use uuid::Uuid;

use crate::error::{CoreError, Result};
use crate::events::{CoreEvent, WeakEventBus};

pub use diff::{ChangeMode, FieldChange};
pub use query::{
    ActivityBatch, ActivityDetails, ActivityEntry, ActivityItem, ActivityPage, ActivityQuery,
    ActivitySession, ActivityStats, ActorFilter, Cursor, SystemEvent, SystemPage, SystemQuery,
};
pub use recorder::{BatchRun, RunBatches};

/// File name of the activity database in the data folder (and in backups).
pub const FILE_NAME: &str = "activity.sqlite";

/// Who made a change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ActorKind {
    /// The user, in Questkeep's own UI.
    User,
    /// A built-in store's sync.
    Store,
    /// A plugin: its store's sync or its extension's calls.
    Plugin,
    /// Questkeep itself: background downloads, auto categories, startup work.
    App,
}

impl ActorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Store => "store",
            Self::Plugin => "plugin",
            Self::App => "app",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "user" => Self::User,
            "store" => Self::Store,
            "plugin" => Self::Plugin,
            "app" => Self::App,
            _ => return None,
        })
    }
}

/// Who made a change: a kind and, for stores and plugins, their id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Actor {
    pub kind: ActorKind,
    /// Store or plugin id; absent for the user and Questkeep.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub id: Option<String>,
}

impl Actor {
    pub fn user() -> Self {
        Self {
            kind: ActorKind::User,
            id: None,
        }
    }

    pub fn app() -> Self {
        Self {
            kind: ActorKind::App,
            id: None,
        }
    }

    pub fn store(id: &str) -> Self {
        Self {
            kind: ActorKind::Store,
            id: Some(id.to_string()),
        }
    }

    pub fn plugin(id: &str) -> Self {
        Self {
            kind: ActorKind::Plugin,
            id: Some(id.to_string()),
        }
    }

    /// The value of the `Questkeep-Actor` IPC header: `plugin:<id>` (ids are lowercase letters,
    /// digits and `-`), or `user`. Anything else is not an actor.
    pub fn from_header(value: &str) -> Option<Self> {
        let value = value.trim();
        if value == "user" {
            return Some(Self::user());
        }
        let id = value.strip_prefix("plugin:")?;
        let valid = !id.is_empty()
            && id.len() <= 64
            && id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
        valid.then(|| Self::plugin(id))
    }

    /// Who syncs a store: the plugin that brings it, else the built-in store.
    pub fn for_provider(provider: &dyn crate::providers::LibraryProvider) -> Self {
        match provider.plugin_id() {
            Some(plugin) => Self::plugin(plugin),
            None => Self::store(provider.id()),
        }
    }
}

/// What kind of change an entry is (the "What" filter).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ActivityKind")]
pub enum EntryKind {
    Added,
    Removed,
    Metadata,
    Edit,
    Images,
    Install,
    Combine,
    Categories,
    Sessions,
    Profiles,
    Scripts,
    Status,
    Other,
}

impl EntryKind {
    pub const ALL: [Self; 13] = [
        Self::Added,
        Self::Removed,
        Self::Metadata,
        Self::Edit,
        Self::Images,
        Self::Install,
        Self::Combine,
        Self::Categories,
        Self::Sessions,
        Self::Profiles,
        Self::Scripts,
        Self::Status,
        Self::Other,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Metadata => "metadata",
            Self::Edit => "edit",
            Self::Images => "images",
            Self::Install => "install",
            Self::Combine => "combine",
            Self::Categories => "categories",
            Self::Sessions => "sessions",
            Self::Profiles => "profiles",
            Self::Scripts => "scripts",
            Self::Status => "status",
            Self::Other => "other",
        }
    }

    pub fn parse(text: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|k| k.as_str() == text)
            .unwrap_or(Self::Other)
    }
}

/// Stable keys of why something changed; the UI turns them into sentences (with `args`).
pub mod reason {
    /// A store sync the user started (`args.trigger`: `manual`, `signIn`, `addGame`, `install`).
    pub const SYNC: &str = "sync";
    /// A store sync Questkeep started (the automatic sync, at startup, after an install folder
    /// changed; `args.trigger`: `startup`, `auto`, `installFolder`).
    pub const AUTO_SYNC: &str = "autoSync";
    /// Metadata the user asked to refresh (`args.sources`).
    pub const METADATA_REFRESH: &str = "metadataRefresh";
    /// Metadata, images and install sizes downloaded in the background.
    pub const METADATA_FETCH: &str = "metadataFetch";
    /// Covers and backgrounds downloaded in the background.
    pub const ARTWORK_FETCH: &str = "artworkFetch";
    /// Artwork the user asked to fetch again from SteamGridDB.
    pub const ARTWORK_REFETCH: &str = "artworkRefetch";
    /// A change in the game's page or edit dialog (`update_game`, the Edit dialog, images).
    pub const EDIT: &str = "edit";
    pub const COMBINE: &str = "combine";
    pub const SEPARATE: &str = "separate";
    pub const PREFER_COPY: &str = "preferCopy";
    /// "Change all" with several games selected.
    pub const BULK_CHANGE: &str = "bulkChange";
    pub const REMOVE: &str = "remove";
    pub const ADD_MANUAL: &str = "addManual";
    pub const SESSION_EDIT: &str = "sessionEdit";
    pub const COMPLETION_STATUS: &str = "completionStatus";
    /// The completion status list changed; games of a removed status moved on.
    pub const COMPLETION_SETTINGS: &str = "completionSettings";
    /// The first start moved a game to "played" (`args.status`).
    pub const FIRST_START: &str = "firstStart";
    /// Categories: `args.name` (and `args.to` for renames and merges).
    pub const CATEGORY_CREATE: &str = "categoryCreate";
    pub const CATEGORY_RENAME: &str = "categoryRename";
    pub const CATEGORY_DELETE: &str = "categoryDelete";
    pub const CATEGORY_MERGE: &str = "categoryMerge";
    pub const CATEGORY_SEPARATE: &str = "categorySeparate";
    pub const CATEGORY_FILTER: &str = "categoryFilter";
    pub const SET_CATEGORIES: &str = "setCategories";
    /// Auto categories given to games that now match their filter.
    pub const AUTO_CATEGORIES: &str = "autoCategories";
    pub const LAUNCH_PROFILES: &str = "launchProfiles";
    /// Scripts: `args.name`.
    pub const SCRIPT_SAVE: &str = "scriptSave";
    pub const SCRIPT_DELETE: &str = "scriptDelete";
    pub const SCRIPT_ORDER: &str = "scriptOrder";
    pub const METADATA_LINK: &str = "metadataLink";
    /// Shown metadata computed again (a new source priority, after an update).
    pub const RESOLVE_ALL: &str = "resolveAll";
    /// `args.createdAt`: when the restored backup was made.
    pub const RESTORE_BACKUP: &str = "restoreBackup";
}

/// The context of a recorded change: who, why, and the batch it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    pub actor: Actor,
    pub reason: String,
    pub args: Option<Value>,
    /// The "What" of the entries; `None` picks one from the changed fields.
    pub kind: Option<EntryKind>,
    pub batch: Option<i64>,
}

impl Change {
    pub fn new(actor: Actor, reason: &str) -> Self {
        Self {
            actor,
            reason: reason.to_string(),
            args: None,
            kind: None,
            batch: None,
        }
    }

    /// Made by whoever the calling thread acts for ([`acting_as`]; the user by default).
    pub fn by_caller(reason: &str) -> Self {
        Self::new(current_actor(), reason)
    }

    pub fn args(mut self, args: Value) -> Self {
        self.args = Some(args);
        self
    }

    pub fn kind(mut self, kind: EntryKind) -> Self {
        self.kind = Some(kind);
        self
    }

    pub fn in_batch(mut self, batch: Option<i64>) -> Self {
        self.batch = batch;
        self
    }
}

/// One entry to write.
#[derive(Debug, Clone, PartialEq)]
pub struct NewEntry {
    pub game_id: Option<Uuid>,
    pub game_name: String,
    pub kind: EntryKind,
    pub changes: Vec<FieldChange>,
    /// Overrides the change's rgs (when they are known only after the change).
    pub args: Option<Value>,
}

thread_local! {
    static DEPTH: Cell<u32> = const { Cell::new(0) };
    static ACTOR: RefCell<Option<Actor>> = const { RefCell::new(None) };
}

/// A recording scope on this thread, see [`in_scope`].
pub(crate) struct Scope(());

impl Scope {
    pub(crate) fn enter() -> Self {
        DEPTH.with(|d| d.set(d.get() + 1));
        Self(())
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
    }
}

/// Whether the calling thread runs inside a recorded change.
pub fn in_scope() -> bool {
    DEPTH.with(|d| d.get() > 0)
}

/// Who the calling thread acts for (see [`acting_as`]); the user unless set.
pub fn current_actor() -> Actor {
    ACTOR
        .with(|a| a.borrow().clone())
        .unwrap_or_else(Actor::user)
}

/// Makes the calling thread act for `actor` until the guard is dropped (the shell sets it from
/// the `Questkeep-Actor` header around each command).
pub fn acting_as(actor: Actor) -> ActingAs {
    let previous = ACTOR.with(|a| a.replace(Some(actor)));
    ActingAs { previous }
}

/// See [`acting_as`].
pub struct ActingAs {
    previous: Option<Actor>,
}

impl Drop for ActingAs {
    fn drop(&mut self) {
        let previous = self.previous.take();
        ACTOR.with(|a| *a.borrow_mut() = previous);
    }
}

/// How much activity is kept (Settings › Activity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ActivitySettings {
    /// The library part may take up to this many megabytes; the oldest entries go first.
    pub library_max_mb: u32,
    /// System events are kept this many days.
    pub system_days: u32,
    /// Record how many requests each plugin made to which host (never what was sent).
    pub record_plugin_requests: bool,
}

impl Default for ActivitySettings {
    fn default() -> Self {
        Self {
            library_max_mb: 50,
            system_days: 30,
            record_plugin_requests: true,
        }
    }
}

/// How bad a system event is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SystemLevel {
    Info,
    Warn,
    Error,
}

impl SystemLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }

    pub fn parse(text: &str) -> Self {
        match text {
            "warn" => Self::Warn,
            "error" => Self::Error,
            _ => Self::Info,
        }
    }
}

/// A system event to write.
#[derive(Debug, Clone, PartialEq)]
pub struct NewSystemEvent {
    pub level: SystemLevel,
    /// `app`, `store` or `plugin`.
    pub source_kind: String,
    pub source_id: Option<String>,
    /// `pluginLoaded`, `http`, `sync`, `backup`, ... (see the System part of the design).
    pub kind: String,
    pub message: String,
    pub details: Option<Value>,
}

impl NewSystemEvent {
    /// Splits a `source` like `plugin:metadata-plus`, `store:steam` or `app`.
    pub fn source(source: &str) -> (String, Option<String>) {
        match source.split_once(':') {
            Some((kind, id)) if !id.is_empty() => (kind.to_string(), Some(id.to_string())),
            _ => (
                if source.is_empty() { "app" } else { source }.to_string(),
                None,
            ),
        }
    }
}

/// Which part of the log "Clear activity…" empties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ActivityPart")]
pub enum ClearPart {
    Library,
    System,
    All,
}

/// How often pruning runs at most (besides after batches).
const PRUNE_EVERY: Duration = Duration::from_secs(60 * 60);

/// The activity database. One connection behind a mutex, like the library's.
pub struct ActivityLog {
    conn: Mutex<Connection>,
    path: Option<PathBuf>,
    events: Mutex<Option<WeakEventBus>>,
    settings: RwLock<ActivitySettings>,
    last_prune: Mutex<Option<Instant>>,
}

impl std::fmt::Debug for ActivityLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActivityLog")
            .field("path", &self.path)
            .finish()
    }
}

pub(crate) fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

pub(crate) fn from_ms(ms: i64) -> DateTime<Utc> {
    DateTime::from_timestamp_millis(ms).unwrap_or_default()
}

fn to_json(value: &Option<Value>) -> Option<String> {
    value.as_ref().map(Value::to_string)
}

impl ActivityLog {
    /// Opens (or creates) the activity database at `path`.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::init(conn, Some(path.to_path_buf()))
    }

    /// Opens the activity database; a broken one is put aside (`.broken`) and started fresh, and
    /// when even that fails the log lives in memory for this run. The library never fails to
    /// open because of its activity log.
    pub fn open_or_fresh(path: &Path) -> Self {
        match Self::open(path) {
            Ok(log) => return log,
            Err(err) => {
                tracing::error!(%err, "could not open the activity log; starting a new one")
            }
        }
        let aside = path.with_extension("sqlite.broken");
        let _ = std::fs::remove_file(&aside);
        if let Err(err) = std::fs::rename(path, &aside) {
            tracing::warn!(%err, "could not put the broken activity log aside");
        }
        for suffix in ["-wal", "-shm"] {
            let mut journal = path.as_os_str().to_owned();
            journal.push(suffix);
            let _ = std::fs::remove_file(PathBuf::from(journal));
        }
        Self::open(path).unwrap_or_else(|err| {
            tracing::error!(%err, "could not create the activity log; keeping it in memory");
            Self::in_memory()
        })
    }

    /// A log in memory (tests, and the last resort of [`Self::open_or_fresh`]).
    pub fn in_memory() -> Self {
        let conn = Connection::open_in_memory().and_then(|c| {
            c.pragma_update(None, "foreign_keys", "ON")?;
            Ok(c)
        });
        let mut conn = match conn {
            Ok(conn) => conn,
            // An in-memory SQLite that can't open means no memory at all.
            Err(err) => panic!("SQLite could not open an in-memory database: {err}"),
        };
        if let Err(err) = migrations::migrate(&mut conn) {
            tracing::error!(%err, "could not set up the in-memory activity log");
        }
        Self::wrap(conn, None)
    }

    fn init(mut conn: Connection, path: Option<PathBuf>) -> Result<Self> {
        // Only takes effect on a new file (before the first table): pages freed by pruning can
        // then be given back with `incremental_vacuum`.
        let pages: i64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0))?;
        if pages == 0 {
            conn.pragma_update(None, "auto_vacuum", "INCREMENTAL")?;
        }
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(Duration::from_secs(5))?;
        migrations::migrate(&mut conn)?;
        Ok(Self::wrap(conn, path))
    }

    fn wrap(conn: Connection, path: Option<PathBuf>) -> Self {
        Self {
            conn: Mutex::new(conn),
            path,
            events: Mutex::new(None),
            settings: RwLock::new(ActivitySettings::default()),
            last_prune: Mutex::new(None),
        }
    }

    pub(crate) fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Where `ActivityAdded` is published.
    pub fn set_events(&self, events: WeakEventBus) {
        *self.events.lock().unwrap_or_else(PoisonError::into_inner) = Some(events);
    }

    pub fn set_settings(&self, settings: ActivitySettings) {
        *self
            .settings
            .write()
            .unwrap_or_else(PoisonError::into_inner) = settings;
    }

    pub fn settings(&self) -> ActivitySettings {
        *self.settings.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn publish(&self, game_ids: Vec<Uuid>) {
        let events = self
            .events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        if let Some(events) = events {
            events.publish(CoreEvent::ActivityAdded { game_ids });
        }
    }

    /// Writes entries in one transaction. A failure is logged and never fails the change itself.
    pub fn write(&self, change: &Change, entries: Vec<NewEntry>) {
        if entries.is_empty() {
            return;
        }
        let game_ids: Vec<Uuid> = entries.iter().filter_map(|e| e.game_id).collect();
        if let Err(err) = self.try_write(change, &entries) {
            tracing::warn!(%err, reason = %change.reason, "could not write to the activity log");
            return;
        }
        self.publish(game_ids);
        if change.batch.is_none() {
            self.prune_if_due();
        }
    }

    fn try_write(&self, change: &Change, entries: &[NewEntry]) -> Result<()> {
        let at = now_ms();
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        {
            let mut entry = tx.prepare_cached(
                "INSERT INTO entries (at, batch_id, actor_kind, actor_id, reason, args_json, \
                 game_id, game_name, kind, change_count) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )?;
            let mut field = tx.prepare_cached(
                "INSERT INTO changes (entry_id, position, field, mode, before_json, after_json) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for new in entries {
                entry.execute(params![
                    at,
                    change.batch,
                    change.actor.kind.as_str(),
                    change.actor.id,
                    change.reason,
                    to_json(if new.args.is_some() {
                        &new.args
                    } else {
                        &change.args
                    }),
                    new.game_id.map(|id| id.to_string()),
                    new.game_name,
                    new.kind.as_str(),
                    i64::try_from(new.changes.len()).unwrap_or(i64::MAX),
                ])?;
                let id = tx.last_insert_rowid();
                for (position, c) in new.changes.iter().enumerate() {
                    let mode = serde_json::to_value(c.mode)?;
                    field.execute(params![
                        id,
                        i64::try_from(position).unwrap_or(i64::MAX),
                        c.field,
                        mode.as_str().unwrap_or("value"),
                        to_json(&c.before),
                        to_json(&c.after),
                    ])?;
                }
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Starts a batch; its entries pass the id in [`Change::batch`]. `None` when it could not be
    /// written (the entries then stand alone).
    pub fn begin_batch(&self, change: &Change) -> Option<i64> {
        let result = (|| -> Result<i64> {
            let conn = self.conn();
            conn.execute(
                "INSERT INTO batches (started_at, actor_kind, actor_id, reason, args_json) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    now_ms(),
                    change.actor.kind.as_str(),
                    change.actor.id,
                    change.reason,
                    to_json(&change.args)
                ],
            )?;
            Ok(conn.last_insert_rowid())
        })();
        result
            .inspect_err(|err| tracing::warn!(%err, "could not start an activity batch"))
            .ok()
    }

    /// Ends a batch with its counts (`added`, `updated`, ...; zeros are left out). A batch that
    /// changed nothing is deleted; one with a single entry and no counts becomes that entry.
    pub fn finish_batch(&self, id: i64, counts: &BTreeMap<String, u64>) {
        let counts: BTreeMap<&String, &u64> = counts.iter().filter(|(_, n)| **n > 0).collect();
        let result = (|| -> Result<bool> {
            let mut conn = self.conn();
            let tx = conn.transaction()?;
            let entries: i64 = tx.query_row(
                "SELECT COUNT(*) FROM entries WHERE batch_id = ?1",
                [id],
                |r| r.get(0),
            )?;
            let kept = if entries == 0 && counts.is_empty() {
                tx.execute("DELETE FROM batches WHERE id = ?1", [id])?;
                false
            } else if entries == 1 && counts.is_empty() {
                tx.execute(
                    "UPDATE entries SET batch_id = NULL WHERE batch_id = ?1",
                    [id],
                )?;
                tx.execute("DELETE FROM batches WHERE id = ?1", [id])?;
                false
            } else {
                tx.execute(
                    "UPDATE batches SET finished_at = ?2, counts_json = ?3 WHERE id = ?1",
                    params![id, now_ms(), serde_json::to_string(&counts)?],
                )?;
                true
            };
            tx.commit()?;
            Ok(kept)
        })();
        match result {
            Ok(true) => self.publish(Vec::new()),
            Ok(false) => {}
            Err(err) => tracing::warn!(%err, "could not finish an activity batch"),
        }
        self.prune_if_due();
    }

    /// Writes one library-level entry (no game), e.g. "Created category X".
    pub fn note(&self, change: &Change, kind: EntryKind, changes: Vec<FieldChange>) {
        self.write(
            change,
            vec![NewEntry {
                game_id: None,
                game_name: String::new(),
                kind,
                changes,
                args: None,
            }],
        );
    }

    /// Writes a system event. Plugin requests are dropped while their recording is off.
    pub fn system_event(&self, event: NewSystemEvent) {
        if event.kind == "http" && !self.settings().record_plugin_requests {
            return;
        }
        let result = self.conn().execute(
            "INSERT INTO system_events (at, level, source_kind, source_id, kind, message, \
             details_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                now_ms(),
                event.level.as_str(),
                event.source_kind,
                event.source_id,
                event.kind,
                event.message,
                to_json(&event.details),
            ],
        );
        if let Err(err) = result {
            // Not through `tracing::warn!` with the activity target: that would loop.
            tracing::debug!(%err, "could not write a system event");
        }
    }

    /// Writes a consistent copy of the log to `path` (which must not exist), for backups.
    pub fn copy_to(&self, path: &Path) -> Result<()> {
        let target = path.to_str().ok_or_else(|| {
            CoreError::Invalid(format!("{} is not a valid path.", path.display()))
        })?;
        self.conn().execute("VACUUM INTO ?1", [target])?;
        Ok(())
    }

    /// Empties a part of the log; the library itself is not touched.
    pub fn clear(&self, part: ClearPart) -> Result<()> {
        {
            let conn = self.conn();
            if matches!(part, ClearPart::Library | ClearPart::All) {
                conn.execute_batch(
                    "DELETE FROM changes; DELETE FROM entries; DELETE FROM batches;",
                )?;
            }
            if matches!(part, ClearPart::System | ClearPart::All) {
                conn.execute("DELETE FROM system_events", [])?;
            }
            give_back(&conn)?;
        }
        self.publish(Vec::new());
        Ok(())
    }

    /// Runs [`Self::prune`] when it did not run for an hour.
    pub fn prune_if_due(&self) {
        {
            let mut last = self
                .last_prune
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if last.is_some_and(|t| t.elapsed() < PRUNE_EVERY) {
                return;
            }
            *last = Some(Instant::now());
        }
        if let Err(err) = self.prune() {
            tracing::warn!(%err, "could not prune the activity log");
        }
    }

    /// Applies the retention settings: system events older than `system_days` go, then the
    /// oldest entries (and their batches) while the library part is larger than
    /// `library_max_mb`. Freed pages are given back to the file system.
    pub fn prune(&self) -> Result<()> {
        let settings = self.settings();
        let limit = u64::from(settings.library_max_mb.max(1)) * 1024 * 1024;
        let conn = self.conn();
        let cutoff = now_ms() - i64::from(settings.system_days) * 24 * 60 * 60 * 1000;
        conn.execute("DELETE FROM system_events WHERE at < ?1", [cutoff])?;
        incremental_vacuum(&conn)?;
        // A few rounds at most: each deletes about the share of entries the limit is exceeded by.
        for _ in 0..16 {
            let sizes = sizes(&conn)?;
            if sizes.library <= limit {
                break;
            }
            let count: i64 = conn.query_row("SELECT COUNT(*) FROM entries", [], |r| r.get(0))?;
            let excess = (sizes.library - limit) as f64 / sizes.library.max(1) as f64;
            // A little more than the excess, so that rounds are few.
            let chunk = ((count as f64 * excess * 1.05).ceil() as i64).clamp(1, count.max(1));
            let deleted = conn.execute(
                "DELETE FROM entries WHERE id IN (SELECT id FROM entries ORDER BY at, id LIMIT ?1)",
                [chunk],
            )?;
            conn.execute(
                "DELETE FROM batches WHERE NOT EXISTS \
                 (SELECT 1 FROM entries WHERE batch_id = batches.id) \
                 AND id NOT IN (SELECT id FROM batches WHERE finished_at IS NULL)",
                [],
            )?;
            incremental_vacuum(&conn)?;
            if deleted == 0 {
                break;
            }
        }
        give_back(&conn)?;
        Ok(())
    }
}

/// Frees the free pages at the end of the file. Each step of the statement frees one page, so it
/// runs to the end (`execute_batch` would stop after the first).
fn incremental_vacuum(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("PRAGMA incremental_vacuum")?;
    let mut rows = stmt.query([])?;
    while rows.next()?.is_some() {}
    Ok(())
}

/// Gives free pages back to the file system: they leave the database file, and the write-ahead
/// log that carried the change is copied in and emptied, so the files on disk shrink now.
fn give_back(conn: &Connection) -> Result<()> {
    incremental_vacuum(conn)?;
    // In memory (or without WAL) this does nothing and answers anyway.
    conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
    Ok(())
}

/// Bytes the library and system parts take: the used pages split by the size of their rows.
pub(crate) struct Sizes {
    pub library: u64,
    pub system: u64,
}

pub(crate) fn sizes(conn: &Connection) -> Result<Sizes> {
    let page_size: i64 = conn.query_row("PRAGMA page_size", [], |r| r.get(0))?;
    let pages: i64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0))?;
    let free: i64 = conn.query_row("PRAGMA freelist_count", [], |r| r.get(0))?;
    let used = u64::try_from((pages - free).max(0) * page_size).unwrap_or_default();
    let system: i64 = conn.query_row(
        "SELECT COALESCE(SUM(length(message) + COALESCE(length(details_json), 0) \
         + length(kind) + length(source_kind) + COALESCE(length(source_id), 0) + 24), 0) \
         FROM system_events",
        [],
        |r| r.get(0),
    )?;
    let library: i64 = conn.query_row(
        "SELECT (SELECT COALESCE(SUM(length(game_name) + length(reason) + \
         COALESCE(length(args_json), 0) + COALESCE(length(game_id), 0) + 48), 0) FROM entries) \
         + (SELECT COALESCE(SUM(length(field) + COALESCE(length(before_json), 0) + \
         COALESCE(length(after_json), 0) + 16), 0) FROM changes) \
         + (SELECT COALESCE(SUM(COALESCE(length(args_json), 0) + \
         COALESCE(length(counts_json), 0) + 48), 0) FROM batches)",
        [],
        |r| r.get(0),
    )?;
    let total = u64::try_from(system.max(0) + library.max(0)).unwrap_or_default();
    let system_share = used
        .saturating_mul(u64::try_from(system.max(0)).unwrap_or_default())
        .checked_div(total)
        .unwrap_or(0);
    Ok(Sizes {
        library: used - system_share,
        system: system_share,
    })
}

#[cfg(test)]
mod tests;
