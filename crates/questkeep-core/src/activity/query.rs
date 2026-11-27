//! Reading the log: the timeline (entries, batches and play sessions merged by time, paged by a
//! keyset cursor), an entry's details, a batch's entries, system events and sizes.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use rusqlite::types::Value as SqlValue;
use rusqlite::{OptionalExtension, Row, params_from_iter};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;
use uuid::Uuid;

use super::{
    ActivityLog, Actor, ActorKind, ChangeMode, EntryKind, FieldChange, SystemLevel, from_ms,
};
use crate::db::Database;
use crate::error::{CoreError, Result};

/// Where a page ends: items older than this come next. Items sort by time, then by kind (a
/// batch before an entry before a session of the same millisecond), then by id, newest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ActivityCursor")]
pub struct Cursor {
    /// Milliseconds since the Unix epoch.
    #[ts(type = "number")]
    pub at: i64,
    pub rank: u8,
    #[ts(type = "number")]
    pub id: i64,
}

const SESSION_RANK: u8 = 0;
const ENTRY_RANK: u8 = 1;
const BATCH_RANK: u8 = 2;

/// Matches one actor, or every actor of a kind when `id` is absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ActivityActorFilter")]
pub struct ActorFilter {
    pub kind: ActorKind,
    #[serde(default)]
    #[ts(optional)]
    pub id: Option<String>,
}

/// What the timeline shows. Empty lists mean "all".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ActivityQuery {
    /// One game's log: its entries (also those inside batches, and those of games combined into
    /// it) and its sessions. Without it: batches stay one item.
    #[ts(optional, type = "string")]
    pub game_id: Option<Uuid>,
    /// Only this batch's entries (e.g. the sync a toast reported).
    #[ts(optional, type = "number")]
    pub batch_id: Option<i64>,
    pub actors: Vec<ActorFilter>,
    pub kinds: Vec<EntryKind>,
    #[ts(optional)]
    pub from: Option<DateTime<Utc>>,
    #[ts(optional)]
    pub to: Option<DateTime<Utc>>,
    /// Part of a game's name.
    #[ts(optional)]
    pub search: Option<String>,
    /// Play sessions are shown (they have no actor or kind: an actor filter without the user
    /// hides them, a kind filter does not).
    pub include_sessions: bool,
    #[ts(optional)]
    pub before: Option<Cursor>,
    /// Items per page (1 to 500).
    pub limit: u32,
}

impl Default for ActivityQuery {
    fn default() -> Self {
        Self {
            game_id: None,
            batch_id: None,
            actors: Vec::new(),
            kinds: Vec::new(),
            from: None,
            to: None,
            search: None,
            include_sessions: true,
            before: None,
            limit: 50,
        }
    }
}

/// One change of one game (or of the library, without a game).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivityEntry {
    #[ts(type = "number")]
    pub id: i64,
    pub at: DateTime<Utc>,
    #[ts(type = "number | null")]
    pub batch_id: Option<i64>,
    pub actor: Actor,
    /// Why (a key, see the reason list); `args` fills in the details.
    pub reason: String,
    #[ts(type = "Record<string, unknown> | null")]
    pub args: Value,
    #[ts(type = "string | null")]
    pub game_id: Option<Uuid>,
    /// The game's name when the entry was written; empty for library-level entries.
    pub game_name: String,
    pub kind: EntryKind,
    /// How many fields changed (`activity_details` lists them).
    pub change_count: u32,
    pub cursor: Cursor,
}

/// Changes made together (a sync, a bulk edit, a fetch run).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivityBatch {
    #[ts(type = "number")]
    pub id: i64,
    pub started_at: DateTime<Utc>,
    /// `None` while it runs.
    pub finished_at: Option<DateTime<Utc>>,
    pub actor: Actor,
    pub reason: String,
    #[ts(type = "Record<string, unknown> | null")]
    pub args: Value,
    /// `added`, `updated`, `removed`, `playtime`, ... (only those above zero).
    #[ts(type = "Record<string, number>")]
    pub counts: BTreeMap<String, u64>,
    pub entry_count: u32,
    /// Games with an entry in it.
    pub game_count: u32,
    pub cursor: Cursor,
}

/// A finished play session, read from the library (not stored twice).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivitySession {
    #[ts(type = "number")]
    pub id: i64,
    /// The library entry.
    #[ts(type = "string")]
    pub game_id: Uuid,
    pub game_name: String,
    /// The copy it ran under, and its store.
    #[ts(type = "string")]
    pub copy_id: Uuid,
    pub store: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    #[ts(type = "number")]
    pub duration_seconds: u64,
    pub cursor: Cursor,
}

/// One line of the timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum ActivityItem {
    Entry(ActivityEntry),
    Batch(ActivityBatch),
    Session(ActivitySession),
}

impl ActivityItem {
    pub fn cursor(&self) -> Cursor {
        match self {
            Self::Entry(e) => e.cursor,
            Self::Batch(b) => b.cursor,
            Self::Session(s) => s.cursor,
        }
    }
}

/// A page of the timeline; `next` asks for the following one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivityPage {
    pub items: Vec<ActivityItem>,
    pub next: Option<Cursor>,
}

/// An entry with its field changes (and its batch).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivityDetails {
    pub entry: ActivityEntry,
    pub changes: Vec<FieldChange>,
    pub batch: Option<ActivityBatch>,
}

/// Something the app or a plugin did behind the scenes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SystemEvent {
    #[ts(type = "number")]
    pub id: i64,
    pub at: DateTime<Utc>,
    pub level: SystemLevel,
    /// `app`, `store` or `plugin`.
    pub source_kind: String,
    /// Store or plugin id.
    pub source_id: Option<String>,
    pub kind: String,
    pub message: String,
    #[ts(type = "Record<string, unknown> | null")]
    pub details: Value,
    pub cursor: Cursor,
}

/// What the System part shows. Empty lists mean "all".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct SystemQuery {
    /// `app`, `store`, `plugin` (all of a kind) or `plugin:<id>`, `store:<id>`.
    pub sources: Vec<String>,
    /// This level and above.
    #[ts(optional)]
    pub min_level: Option<SystemLevel>,
    pub kinds: Vec<String>,
    #[ts(optional)]
    pub from: Option<DateTime<Utc>>,
    #[ts(optional)]
    pub to: Option<DateTime<Utc>>,
    /// Part of the message.
    #[ts(optional)]
    pub search: Option<String>,
    #[ts(optional)]
    pub before: Option<Cursor>,
    pub limit: u32,
}

impl Default for SystemQuery {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            min_level: None,
            kinds: Vec::new(),
            from: None,
            to: None,
            search: None,
            before: None,
            limit: 50,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SystemPage {
    pub items: Vec<SystemEvent>,
    pub next: Option<Cursor>,
}

/// How large the log is (Settings › Activity).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivityStats {
    /// Bytes of the library part (entries, batches, changes).
    #[ts(type = "number")]
    pub library_bytes: u64,
    /// Bytes of system events.
    #[ts(type = "number")]
    pub system_bytes: u64,
    /// The file on disk (with free pages not given back yet).
    #[ts(type = "number")]
    pub file_bytes: u64,
    #[ts(type = "number")]
    pub entries: u64,
    #[ts(type = "number")]
    pub system_events: u64,
    pub oldest_entry: Option<DateTime<Utc>>,
}

/// Filters for the sessions the timeline reads from the library.
#[derive(Debug, Clone, Default)]
pub(crate) struct SessionFilter {
    /// Library entries (and their copies); `None`: all.
    pub entries: Option<Vec<Uuid>>,
    pub from_ms: Option<i64>,
    pub to_ms: Option<i64>,
    pub search: Option<String>,
    pub before: Option<Cursor>,
    pub limit: u32,
}

fn like(text: &str) -> String {
    let escaped = text
        .trim()
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}

/// SQL that keeps items of `rank` older than the cursor (`at`/`id` are the item's columns).
pub(crate) fn before_sql(
    cursor: Option<Cursor>,
    rank: u8,
    at: &str,
    id: &str,
    args: &mut Vec<SqlValue>,
) -> String {
    let Some(c) = cursor else {
        return String::new();
    };
    args.push(SqlValue::Integer(c.at));
    let a = args.len();
    let same = match rank.cmp(&c.rank) {
        std::cmp::Ordering::Less => "1".to_string(),
        std::cmp::Ordering::Greater => "0".to_string(),
        std::cmp::Ordering::Equal => {
            args.push(SqlValue::Integer(c.id));
            format!("{id} < ?{}", args.len())
        }
    };
    format!(" AND ({at} < ?{a} OR ({at} = ?{a} AND {same}))")
}

fn actor_sql(actors: &[ActorFilter], prefix: &str, args: &mut Vec<SqlValue>) -> String {
    if actors.is_empty() {
        return String::new();
    }
    let parts: Vec<String> = actors
        .iter()
        .map(|a| {
            args.push(SqlValue::Text(a.kind.as_str().into()));
            let kind = args.len();
            match &a.id {
                Some(id) => {
                    args.push(SqlValue::Text(id.clone()));
                    format!(
                        "({prefix}actor_kind = ?{kind} AND {prefix}actor_id = ?{})",
                        args.len()
                    )
                }
                None => format!("{prefix}actor_kind = ?{kind}"),
            }
        })
        .collect();
    format!(" AND ({})", parts.join(" OR "))
}

fn kinds_sql(kinds: &[EntryKind], prefix: &str, args: &mut Vec<SqlValue>) -> String {
    if kinds.is_empty() {
        return String::new();
    }
    let marks: Vec<String> = kinds
        .iter()
        .map(|k| {
            args.push(SqlValue::Text(k.as_str().into()));
            format!("?{}", args.len())
        })
        .collect();
    format!(" AND {prefix}kind IN ({})", marks.join(", "))
}

fn time_sql(
    query_from: Option<i64>,
    to: Option<i64>,
    at: &str,
    args: &mut Vec<SqlValue>,
) -> String {
    let mut sql = String::new();
    if let Some(from) = query_from {
        args.push(SqlValue::Integer(from));
        sql.push_str(&format!(" AND {at} >= ?{}", args.len()));
    }
    if let Some(to) = to {
        args.push(SqlValue::Integer(to));
        sql.push_str(&format!(" AND {at} < ?{}", args.len()));
    }
    sql
}

fn search_sql(search: Option<&str>, column: &str, args: &mut Vec<SqlValue>) -> String {
    match search.map(str::trim).filter(|s| !s.is_empty()) {
        Some(text) => {
            args.push(SqlValue::Text(like(text)));
            format!(" AND {column} LIKE ?{} ESCAPE '\\'", args.len())
        }
        None => String::new(),
    }
}

fn json_or_null(text: Option<String>) -> Value {
    text.and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(Value::Null)
}

fn actor_of(kind: &str, id: Option<String>) -> Actor {
    Actor {
        kind: ActorKind::parse(kind).unwrap_or(ActorKind::App),
        id,
    }
}

const ENTRY_COLUMNS: &str = "e.id, e.at, e.batch_id, e.actor_kind, e.actor_id, e.reason, e.args_json, e.game_id, \
     e.game_name, e.kind, e.change_count";

fn entry_row(r: &Row<'_>) -> rusqlite::Result<ActivityEntry> {
    let id: i64 = r.get(0)?;
    let at: i64 = r.get(1)?;
    Ok(ActivityEntry {
        id,
        at: from_ms(at),
        batch_id: r.get(2)?,
        actor: actor_of(&r.get::<_, String>(3)?, r.get(4)?),
        reason: r.get(5)?,
        args: json_or_null(r.get(6)?),
        game_id: r
            .get::<_, Option<String>>(7)?
            .and_then(|g| Uuid::parse_str(&g).ok()),
        game_name: r.get(8)?,
        kind: EntryKind::parse(&r.get::<_, String>(9)?),
        change_count: u32::try_from(r.get::<_, i64>(10)?).unwrap_or_default(),
        cursor: Cursor {
            at,
            rank: ENTRY_RANK,
            id,
        },
    })
}

const BATCH_COLUMNS: &str = "b.id, b.started_at, b.finished_at, b.actor_kind, b.actor_id, \
     b.reason, b.args_json, b.counts_json, \
     (SELECT COUNT(*) FROM entries x WHERE x.batch_id = b.id), \
     (SELECT COUNT(DISTINCT x.game_id) FROM entries x WHERE x.batch_id = b.id)";

fn batch_row(r: &Row<'_>) -> rusqlite::Result<ActivityBatch> {
    let id: i64 = r.get(0)?;
    let at: i64 = r.get(1)?;
    Ok(ActivityBatch {
        id,
        started_at: from_ms(at),
        finished_at: r.get::<_, Option<i64>>(2)?.map(from_ms),
        actor: actor_of(&r.get::<_, String>(3)?, r.get(4)?),
        reason: r.get(5)?,
        args: json_or_null(r.get(6)?),
        counts: r
            .get::<_, Option<String>>(7)?
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default(),
        entry_count: u32::try_from(r.get::<_, i64>(8)?).unwrap_or_default(),
        game_count: u32::try_from(r.get::<_, i64>(9)?).unwrap_or_default(),
        cursor: Cursor {
            at,
            rank: BATCH_RANK,
            id,
        },
    })
}

fn limit_of(limit: u32) -> u32 {
    limit.clamp(1, 500)
}

/// Sorts newest first, cuts to `limit` and says where the next page starts.
fn page<T>(mut items: Vec<T>, limit: u32, key: impl Fn(&T) -> Cursor) -> (Vec<T>, Option<Cursor>) {
    items.sort_by_key(|item| std::cmp::Reverse(key(item)));
    let limit = limit as usize;
    let more = items.len() > limit;
    items.truncate(limit);
    let next = if more { items.last().map(key) } else { None };
    (items, next)
}

impl ActivityLog {
    /// Entries matching the query; `game_ids` limits them to these games (a game's log).
    fn entries(
        &self,
        query: &ActivityQuery,
        game_ids: Option<&[Uuid]>,
        limit: u32,
    ) -> Result<Vec<ActivityEntry>> {
        let mut args: Vec<SqlValue> = Vec::new();
        let mut sql = format!("SELECT {ENTRY_COLUMNS} FROM entries e WHERE 1");
        match (query.batch_id, game_ids) {
            (Some(batch), _) => {
                args.push(SqlValue::Integer(batch));
                sql.push_str(&format!(" AND e.batch_id = ?{}", args.len()));
            }
            (None, Some(ids)) => {
                let marks: Vec<String> = ids
                    .iter()
                    .map(|id| {
                        args.push(SqlValue::Text(id.to_string()));
                        format!("?{}", args.len())
                    })
                    .collect();
                sql.push_str(&format!(" AND e.game_id IN ({})", marks.join(", ")));
            }
            (None, None) => sql.push_str(" AND e.batch_id IS NULL"),
        }
        sql.push_str(&actor_sql(&query.actors, "e.", &mut args));
        sql.push_str(&kinds_sql(&query.kinds, "e.", &mut args));
        sql.push_str(&time_sql(
            query.from.map(|t| t.timestamp_millis()),
            query.to.map(|t| t.timestamp_millis()),
            "e.at",
            &mut args,
        ));
        sql.push_str(&search_sql(
            query.search.as_deref(),
            "e.game_name",
            &mut args,
        ));
        sql.push_str(&before_sql(
            query.before,
            ENTRY_RANK,
            "e.at",
            "e.id",
            &mut args,
        ));
        args.push(SqlValue::Integer(i64::from(limit)));
        sql.push_str(&format!(
            " ORDER BY e.at DESC, e.id DESC LIMIT ?{}",
            args.len()
        ));
        let conn = self.conn();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params_from_iter(args), entry_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Batches matching the query (the global log only).
    fn batches(&self, query: &ActivityQuery, limit: u32) -> Result<Vec<ActivityBatch>> {
        let mut args: Vec<SqlValue> = Vec::new();
        // A running batch shows once it has an entry.
        let mut sql = format!(
            "SELECT {BATCH_COLUMNS} FROM batches b WHERE (b.finished_at IS NOT NULL \
             OR EXISTS (SELECT 1 FROM entries x WHERE x.batch_id = b.id))"
        );
        sql.push_str(&actor_sql(&query.actors, "b.", &mut args));
        sql.push_str(&time_sql(
            query.from.map(|t| t.timestamp_millis()),
            query.to.map(|t| t.timestamp_millis()),
            "b.started_at",
            &mut args,
        ));
        // A kind or name filter keeps the batches with a matching entry.
        let mut inner = String::new();
        inner.push_str(&kinds_sql(&query.kinds, "x.", &mut args));
        inner.push_str(&search_sql(
            query.search.as_deref(),
            "x.game_name",
            &mut args,
        ));
        if !inner.is_empty() {
            sql.push_str(&format!(
                " AND EXISTS (SELECT 1 FROM entries x WHERE x.batch_id = b.id{inner})"
            ));
        }
        sql.push_str(&before_sql(
            query.before,
            BATCH_RANK,
            "b.started_at",
            "b.id",
            &mut args,
        ));
        args.push(SqlValue::Integer(i64::from(limit)));
        sql.push_str(&format!(
            " ORDER BY b.started_at DESC, b.id DESC LIMIT ?{}",
            args.len()
        ));
        let conn = self.conn();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params_from_iter(args), batch_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// One batch.
    pub fn batch(&self, id: i64) -> Result<Option<ActivityBatch>> {
        let conn = self.conn();
        Ok(conn
            .query_row(
                &format!("SELECT {BATCH_COLUMNS} FROM batches b WHERE b.id = ?1"),
                [id],
                batch_row,
            )
            .optional()?)
    }

    /// An entry with its field changes and its batch.
    pub fn details(&self, entry_id: i64) -> Result<ActivityDetails> {
        let entry = {
            let conn = self.conn();
            conn.query_row(
                &format!("SELECT {ENTRY_COLUMNS} FROM entries e WHERE e.id = ?1"),
                [entry_id],
                entry_row,
            )
            .optional()?
            .ok_or_else(|| CoreError::Invalid("This activity entry no longer exists.".into()))?
        };
        let changes = {
            let conn = self.conn();
            let mut stmt = conn.prepare(
                "SELECT field, mode, before_json, after_json FROM changes \
                 WHERE entry_id = ?1 ORDER BY position",
            )?;
            stmt.query_map([entry_id], |r| {
                let mode: String = r.get(1)?;
                Ok(FieldChange {
                    field: r.get(0)?,
                    mode: serde_json::from_value(Value::String(mode)).unwrap_or(ChangeMode::Value),
                    before: r
                        .get::<_, Option<String>>(2)?
                        .and_then(|t| serde_json::from_str(&t).ok()),
                    after: r
                        .get::<_, Option<String>>(3)?
                        .and_then(|t| serde_json::from_str(&t).ok()),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
        };
        let batch = match entry.batch_id {
            Some(id) => self.batch(id)?,
            None => None,
        };
        Ok(ActivityDetails {
            entry,
            changes,
            batch,
        })
    }

    /// The entries of a batch, newest first.
    pub fn batch_entries(
        &self,
        batch_id: i64,
        before: Option<Cursor>,
        limit: u32,
    ) -> Result<ActivityPage> {
        let limit = limit_of(limit);
        let query = ActivityQuery {
            batch_id: Some(batch_id),
            before,
            ..ActivityQuery::default()
        };
        let entries = self.entries(&query, None, limit + 1)?;
        let (items, next) = page(
            entries.into_iter().map(ActivityItem::Entry).collect(),
            limit,
            ActivityItem::cursor,
        );
        Ok(ActivityPage { items, next })
    }

    /// One page of the timeline. Sessions come from the library database `db`.
    pub fn timeline(&self, db: &Database, query: &ActivityQuery) -> Result<ActivityPage> {
        let limit = limit_of(query.limit);
        let fetch = limit + 1;
        // A game's log covers the games combined into it (their history before combining).
        let game_ids: Option<Vec<Uuid>> = query.game_id.map(|id| match db.get_entry(id) {
            Ok(entry) => {
                let mut ids: Vec<Uuid> = entry.copies.iter().map(|c| c.id).collect();
                if !ids.contains(&entry.id) {
                    ids.push(entry.id);
                }
                if !ids.contains(&id) {
                    ids.push(id);
                }
                ids
            }
            // A removed game: its own entries still read.
            Err(_) => vec![id],
        });
        let mut items: Vec<ActivityItem> = self
            .entries(query, game_ids.as_deref(), fetch)?
            .into_iter()
            .map(ActivityItem::Entry)
            .collect();
        if query.game_id.is_none() && query.batch_id.is_none() {
            items.extend(
                self.batches(query, fetch)?
                    .into_iter()
                    .map(ActivityItem::Batch),
            );
        }
        let sessions_wanted = query.include_sessions
            && query.batch_id.is_none()
            && (query.actors.is_empty() || query.actors.iter().any(|a| a.kind == ActorKind::User));
        if sessions_wanted {
            let filter = SessionFilter {
                entries: game_ids.clone(),
                from_ms: query.from.map(|t| t.timestamp_millis()),
                to_ms: query.to.map(|t| t.timestamp_millis()),
                search: query.search.clone(),
                before: query.before,
                limit: fetch,
            };
            items.extend(
                db.activity_sessions(&filter)?
                    .into_iter()
                    .map(ActivityItem::Session),
            );
        }
        let (items, next) = page(items, limit, ActivityItem::cursor);
        Ok(ActivityPage { items, next })
    }

    /// One page of system events.
    pub fn system_events(&self, query: &SystemQuery) -> Result<SystemPage> {
        let limit = limit_of(query.limit);
        let mut args: Vec<SqlValue> = Vec::new();
        let mut sql = String::from(
            "SELECT id, at, level, source_kind, source_id, kind, message, details_json \
             FROM system_events WHERE 1",
        );
        if !query.sources.is_empty() {
            let parts: Vec<String> = query
                .sources
                .iter()
                .map(|source| {
                    let (kind, id) = super::NewSystemEvent::source(source);
                    args.push(SqlValue::Text(kind));
                    let k = args.len();
                    match id {
                        Some(id) => {
                            args.push(SqlValue::Text(id));
                            format!("(source_kind = ?{k} AND source_id = ?{})", args.len())
                        }
                        None => format!("source_kind = ?{k}"),
                    }
                })
                .collect();
            sql.push_str(&format!(" AND ({})", parts.join(" OR ")));
        }
        if let Some(min) = query.min_level {
            let levels: Vec<String> = [SystemLevel::Info, SystemLevel::Warn, SystemLevel::Error]
                .into_iter()
                .filter(|l| *l >= min)
                .map(|l| {
                    args.push(SqlValue::Text(l.as_str().into()));
                    format!("?{}", args.len())
                })
                .collect();
            sql.push_str(&format!(" AND level IN ({})", levels.join(", ")));
        }
        if !query.kinds.is_empty() {
            let marks: Vec<String> = query
                .kinds
                .iter()
                .map(|k| {
                    args.push(SqlValue::Text(k.clone()));
                    format!("?{}", args.len())
                })
                .collect();
            sql.push_str(&format!(" AND kind IN ({})", marks.join(", ")));
        }
        sql.push_str(&time_sql(
            query.from.map(|t| t.timestamp_millis()),
            query.to.map(|t| t.timestamp_millis()),
            "at",
            &mut args,
        ));
        sql.push_str(&search_sql(query.search.as_deref(), "message", &mut args));
        sql.push_str(&before_sql(query.before, ENTRY_RANK, "at", "id", &mut args));
        args.push(SqlValue::Integer(i64::from(limit + 1)));
        sql.push_str(&format!(" ORDER BY at DESC, id DESC LIMIT ?{}", args.len()));
        let conn = self.conn();
        let mut stmt = conn.prepare(&sql)?;
        let events = stmt
            .query_map(params_from_iter(args), |r| {
                let id: i64 = r.get(0)?;
                let at: i64 = r.get(1)?;
                Ok(SystemEvent {
                    id,
                    at: from_ms(at),
                    level: SystemLevel::parse(&r.get::<_, String>(2)?),
                    source_kind: r.get(3)?,
                    source_id: r.get(4)?,
                    kind: r.get(5)?,
                    message: r.get(6)?,
                    details: json_or_null(r.get(7)?),
                    cursor: Cursor {
                        at,
                        rank: ENTRY_RANK,
                        id,
                    },
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);
        drop(conn);
        let (items, next) = page(events, limit, |e| e.cursor);
        Ok(SystemPage { items, next })
    }

    /// What the log's files take on disk (the database and its write-ahead log); `None` in
    /// memory.
    fn disk_bytes(&self) -> Option<u64> {
        let path = self.path.as_ref()?;
        let size = |p: &std::path::Path| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
        let mut wal = path.clone().into_os_string();
        wal.push("-wal");
        Some(size(path) + size(std::path::Path::new(&wal)))
    }

    /// Sizes and counts for Settings › Activity.
    pub fn stats(&self) -> Result<ActivityStats> {
        let conn = self.conn();
        let sizes = super::sizes(&conn)?;
        let page_size: i64 = conn.query_row("PRAGMA page_size", [], |r| r.get(0))?;
        let pages: i64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0))?;
        let count = |table: &str| -> Result<u64> {
            let n: i64 =
                conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))?;
            Ok(u64::try_from(n).unwrap_or_default())
        };
        let oldest: Option<i64> =
            conn.query_row("SELECT MIN(at) FROM entries", [], |r| r.get(0))?;
        Ok(ActivityStats {
            library_bytes: sizes.library,
            system_bytes: sizes.system,
            file_bytes: self
                .disk_bytes()
                .unwrap_or_else(|| u64::try_from(pages * page_size).unwrap_or_default()),
            entries: count("entries")?,
            system_events: count("system_events")?,
            oldest_entry: oldest.map(from_ms),
        })
    }
}

impl Database {
    /// Finished play sessions for the timeline, newest first, with their entry's name.
    pub(crate) fn activity_sessions(&self, filter: &SessionFilter) -> Result<Vec<ActivitySession>> {
        // Session times are stored as text; the timeline sorts by milliseconds.
        const AT: &str =
            "CAST(round((julianday(s.started_at) - 2440587.5) * 86400000.0) AS INTEGER)";
        let mut args: Vec<SqlValue> = Vec::new();
        let mut sql = format!(
            "SELECT s.id, s.game_id, e.id, e.name, g.source, s.started_at, s.ended_at, \
             s.duration_seconds, {AT} AS at_ms \
             FROM play_sessions s JOIN games g ON g.id = s.game_id \
             JOIN games e ON e.id = COALESCE(g.merged_into, g.id) \
             WHERE s.ended_at IS NOT NULL"
        );
        if let Some(entries) = &filter.entries {
            let marks: Vec<String> = entries
                .iter()
                .map(|id| {
                    args.push(SqlValue::Text(id.to_string()));
                    format!("?{}", args.len())
                })
                .collect();
            sql.push_str(&format!(" AND e.id IN ({})", marks.join(", ")));
        }
        sql.push_str(&time_sql(filter.from_ms, filter.to_ms, AT, &mut args));
        sql.push_str(&search_sql(filter.search.as_deref(), "e.name", &mut args));
        sql.push_str(&before_sql(
            filter.before,
            SESSION_RANK,
            AT,
            "s.id",
            &mut args,
        ));
        args.push(SqlValue::Integer(i64::from(filter.limit)));
        sql.push_str(&format!(
            " ORDER BY at_ms DESC, s.id DESC LIMIT ?{}",
            args.len()
        ));
        let conn = self.conn();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(args), |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, DateTime<Utc>>(5)?,
                r.get::<_, DateTime<Utc>>(6)?,
                r.get::<_, Option<i64>>(7)?,
                r.get::<_, i64>(8)?,
            ))
        })?;
        let mut sessions = Vec::new();
        for row in rows {
            let (id, copy, entry, name, store, started_at, ended_at, credited, at) = row?;
            let (Ok(copy_id), Ok(game_id)) = (Uuid::parse_str(&copy), Uuid::parse_str(&entry))
            else {
                continue;
            };
            let seconds = credited.unwrap_or_else(|| (ended_at - started_at).num_seconds());
            sessions.push(ActivitySession {
                id,
                game_id,
                game_name: name,
                copy_id,
                store,
                started_at,
                ended_at,
                duration_seconds: u64::try_from(seconds).unwrap_or_default(),
                cursor: Cursor {
                    at,
                    rank: SESSION_RANK,
                    id,
                },
            });
        }
        Ok(sessions)
    }
}
