//! The recorder: write sites wrap their mutation in [`Database::record`] (or
//! [`Database::record_all`] for bulk changes); the affected entries are snapshotted before and
//! after, and one entry per game that visibly changed is written to the activity log. The
//! mutation's result comes back unchanged; problems with the log are only logged.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::Value;
use uuid::Uuid;

use super::diff::{self, EntryExtras, EntrySnapshot, FieldChange, ProfileFacts};
use super::{ActivityLog, Actor, Change, EntryKind, NewEntry, Scope};
use crate::db::Database;
use crate::error::Result;
use crate::model::{GameQuery, HiddenFilter, KindFilter};

/// Above this many games a snapshot reads the whole library in one query instead of game by game.
const BULK_SNAPSHOT_FROM: usize = 48;

/// The shown state of several entries, in library order.
#[derive(Debug, Default)]
pub(crate) struct Snapshot {
    order: Vec<Uuid>,
    entries: HashMap<Uuid, EntrySnapshot>,
}

/// What a recorded change did, for the counts of its batch.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Recorded {
    /// Entries written.
    pub entries: u64,
    /// Entries whose playtime changed (store playtime of a sync; not shown per game).
    pub playtime: u64,
}

impl Database {
    /// Reads the shown state of `entries` (library entry ids), or of the whole library.
    pub(crate) fn snapshot(&self, entries: Option<&[Uuid]>) -> Result<Snapshot> {
        let games = match entries {
            Some(ids) if ids.len() < BULK_SNAPSHOT_FROM => {
                let mut games = Vec::with_capacity(ids.len());
                for id in ids {
                    if let Ok(game) = self.get_entry(*id) {
                        games.push(game);
                    }
                }
                games
            }
            _ => {
                let all = self.list_games(&GameQuery {
                    hidden: HiddenFilter::All,
                    kind: KindFilter::Both,
                    ..GameQuery::default()
                })?;
                match entries {
                    Some(ids) => {
                        let wanted: HashSet<&Uuid> = ids.iter().collect();
                        all.into_iter().filter(|g| wanted.contains(&g.id)).collect()
                    }
                    None => all,
                }
            }
        };
        let (categories, extras) = self.snapshot_extras()?;
        let mut snapshot = Snapshot::default();
        for game in games {
            let extra = extras.get(&game.id).cloned().unwrap_or_default();
            snapshot.order.push(game.id);
            snapshot
                .entries
                .insert(game.id, EntrySnapshot::of(&game, &categories, &extra));
        }
        Ok(snapshot)
    }

    /// Category names, and per entry what the snapshot needs besides the game row.
    fn snapshot_extras(&self) -> Result<(HashMap<String, String>, HashMap<Uuid, EntryExtras>)> {
        let conn = self.conn();
        let mut categories = HashMap::new();
        {
            let mut stmt = conn.prepare_cached("SELECT id, name FROM categories")?;
            let rows =
                stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
            for row in rows {
                let (id, name) = row?;
                categories.insert(id, name);
            }
        }
        let mut extras: HashMap<Uuid, EntryExtras> = HashMap::new();
        {
            let mut stmt = conn.prepare_cached(
                "SELECT id, sorting_name_user, default_profile FROM games \
                 WHERE merged_into IS NULL \
                 AND (sorting_name_user IS NOT NULL OR default_profile IS NOT NULL)",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            })?;
            for row in rows {
                let (id, sorting, default) = row?;
                if let Ok(id) = Uuid::parse_str(&id) {
                    let extra = extras.entry(id).or_default();
                    extra.user_sorting_name = sorting;
                    extra.default_profile = default;
                }
            }
        }
        {
            // Own profiles by their entry, a copy's changed profile by the entry of the copy.
            let mut stmt = conn.prepare_cached(
                "SELECT p.id, COALESCE(p.entry_id, (SELECT COALESCE(g.merged_into, g.id) \
                 FROM games g WHERE g.id = p.id)), p.name, p.args, \
                 json_extract(p.action, '$.target') \
                 FROM launch_profiles p ORDER BY p.position, p.rowid",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                ))
            })?;
            for row in rows {
                let (id, entry, name, args, target) = row?;
                if let Some(entry) = entry.and_then(|e| Uuid::parse_str(&e).ok()) {
                    extras
                        .entry(entry)
                        .or_default()
                        .profiles
                        .push(ProfileFacts {
                            id,
                            name,
                            args,
                            target,
                        });
                }
            }
        }
        Ok((categories, extras))
    }

    /// The entries `ids` belong to right now (ids of removed games are left out).
    fn entries_of(&self, ids: &[Uuid]) -> Vec<Uuid> {
        let mut seen = HashSet::new();
        ids.iter()
            .filter_map(|id| self.entry_of(*id).ok())
            .filter(|id| seen.insert(*id))
            .collect()
    }

    /// Runs `mutate`, a change of the games `ids` (copies or entries), and records what visibly
    /// changed. Use [`Self::record_new`] when the change adds games.
    pub fn record<T>(
        &self,
        change: &Change,
        ids: &[Uuid],
        mutate: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        self.record_inner(change, Some(ids), mutate, |_| Vec::new(), true)
            .map(|(value, _)| value)
    }

    /// [`Self::record`] for a change that adds games: `added` names them from the result.
    pub fn record_new<T>(
        &self,
        change: &Change,
        ids: &[Uuid],
        mutate: impl FnOnce() -> Result<T>,
        added: impl FnOnce(&T) -> Vec<Uuid>,
    ) -> Result<T> {
        self.record_inner(change, Some(ids), mutate, added, true)
            .map(|(value, _)| value)
    }

    /// Runs a change that may touch any game (a sync, auto categories, computing all metadata)
    /// with one snapshot of the whole library before and after.
    pub fn record_all<T>(
        &self,
        change: &Change,
        mutate: impl FnOnce() -> Result<T>,
    ) -> Result<(T, Recorded)> {
        self.record_inner(change, None, mutate, |_| Vec::new(), true)
    }

    /// Runs `mutate` as a recorded change whose entries the caller writes itself (scripts,
    /// sessions, library-level changes): no snapshots, just the recording scope.
    pub fn record_explicit<T>(
        &self,
        change: &Change,
        mutate: impl FnOnce() -> Result<T>,
        entries: impl FnOnce(&T) -> Vec<NewEntry>,
    ) -> Result<T> {
        let _turn = self.recording_turn();
        let _scope = Scope::enter();
        let value = mutate()?;
        if let Some(log) = self.activity() {
            log.write(change, entries(&value));
        }
        Ok(value)
    }

    /// [`Self::record`] for a change that waits on the network (a fetch job). It does not take
    /// the recording turn, so other changes go on meanwhile; one another thread makes to the same
    /// game in that time shows in this entry too.
    pub fn record_long<T>(
        &self,
        change: &Change,
        ids: &[Uuid],
        mutate: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        self.record_inner(change, Some(ids), mutate, |_| Vec::new(), false)
            .map(|(value, _)| value)
    }

    fn record_inner<T>(
        &self,
        change: &Change,
        ids: Option<&[Uuid]>,
        mutate: impl FnOnce() -> Result<T>,
        added: impl FnOnce(&T) -> Vec<Uuid>,
        turn: bool,
    ) -> Result<(T, Recorded)> {
        // Waits while another thread records, so its change never shows in these snapshots.
        let _turn = if turn { self.recording_turn() } else { None };
        let _scope = Scope::enter();
        let Some(log) = self.activity().cloned() else {
            return mutate().map(|value| (value, Recorded::default()));
        };
        let targets = ids.map(|ids| self.entries_of(ids));
        let before = self
            .snapshot(targets.as_deref())
            .inspect_err(|err| tracing::warn!(%err, "could not read the games before a change"))
            .ok();
        let value = mutate()?;
        let Some(before) = before else {
            return Ok((value, Recorded::default()));
        };
        let after_targets = ids.map(|ids| {
            let mut all: Vec<Uuid> = before.order.clone();
            all.extend(added(&value));
            all.extend(ids.iter().copied());
            self.entries_of(&all)
        });
        let after = match self.snapshot(after_targets.as_deref()) {
            Ok(after) => after,
            Err(err) => {
                tracing::warn!(%err, "could not read the games after a change");
                return Ok((value, Recorded::default()));
            }
        };
        let (entries, playtime) = entries_between(change, &before, &after);
        let recorded = Recorded {
            entries: entries.len() as u64,
            playtime,
        };
        log.write(change, entries);
        Ok((value, recorded))
    }

    /// [`Self::record_all`] inside a batch of its own; `note` (a library-level entry, e.g. the
    /// deleted category) joins the batch when the change succeeded.
    pub fn record_batch<T>(
        &self,
        change: &Change,
        note: Option<NewEntry>,
        mutate: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        let batch = self.begin_batch(change);
        let (value, _) = self.record_all(batch.change(), mutate)?;
        if let (Some(note), Some(log)) = (note, self.activity()) {
            log.write(batch.change(), vec![note]);
        }
        batch.finish();
        Ok(value)
    }

    /// [`Self::record`] for a change of several games at once: their entries form a batch.
    pub fn record_group<T>(
        &self,
        change: &Change,
        ids: &[Uuid],
        mutate: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        if ids.len() < 2 {
            return self.record(change, ids, mutate);
        }
        let batch = self.begin_batch(change);
        let value = self.record(batch.change(), ids, mutate)?;
        batch.finish();
        Ok(value)
    }

    /// Starts a batch for `change` (a sync, a bulk edit); record its changes with
    /// [`BatchRun::change`] and finish it with its counts.
    pub fn begin_batch(&self, change: &Change) -> BatchRun {
        let log = self.activity().cloned();
        let id = log.as_ref().and_then(|log| log.begin_batch(change));
        BatchRun {
            log,
            change: change.clone().in_batch(id),
            counts: BTreeMap::new(),
            finished: false,
        }
    }
}

/// The "What" from the changed fields; plain field changes are edits when a person made them.
fn default_kind(change: &Change, changes: &[FieldChange]) -> EntryKind {
    match diff::kind_of(changes) {
        EntryKind::Metadata
            if matches!(
                change.actor.kind,
                super::ActorKind::User | super::ActorKind::Plugin
            ) && ![
                super::reason::SYNC,
                super::reason::AUTO_SYNC,
                super::reason::METADATA_REFRESH,
                super::reason::METADATA_FETCH,
            ]
            .contains(&change.reason.as_str()) =>
        {
            EntryKind::Edit
        }
        kind => kind,
    }
}

/// The entries that describe going from `before` to `after`, and how many entries' playtime
/// changed.
fn entries_between(change: &Change, before: &Snapshot, after: &Snapshot) -> (Vec<NewEntry>, u64) {
    let combining = change.kind == Some(EntryKind::Combine);
    let mut entries = Vec::new();
    let mut playtime = 0;
    for id in &after.order {
        let Some(new) = after.entries.get(id) else {
            continue;
        };
        let entry = match before.entries.get(id) {
            Some(old) => {
                if old.playtime != new.playtime {
                    playtime += 1;
                }
                let changes = diff::diff(old, new);
                if changes.is_empty() {
                    continue;
                }
                NewEntry {
                    game_id: Some(*id),
                    game_name: new.name.clone(),
                    kind: change
                        .kind
                        .unwrap_or_else(|| default_kind(change, &changes)),
                    changes,
                    args: None,
                }
            }
            // A copy taken out of its entry becomes an entry of its own.
            None => NewEntry {
                game_id: Some(*id),
                game_name: new.name.clone(),
                kind: if combining {
                    EntryKind::Combine
                } else {
                    EntryKind::Added
                },
                changes: diff::added(new),
                args: None,
            },
        };
        entries.push(entry);
    }
    // The entry everything was combined into, for the absorbed ones.
    let into = after.order.first().and_then(|id| after.entries.get(id));
    for id in &before.order {
        if after.entries.contains_key(id) {
            continue;
        }
        let Some(old) = before.entries.get(id) else {
            continue;
        };
        let entry = if combining {
            NewEntry {
                game_id: Some(*id),
                game_name: old.name.clone(),
                kind: EntryKind::Combine,
                changes: vec![FieldChange::value(
                    "combinedInto",
                    None,
                    into.map(|e| Value::String(e.name.clone())),
                )],
                args: None,
            }
        } else {
            NewEntry {
                game_id: Some(*id),
                game_name: old.name.clone(),
                kind: EntryKind::Removed,
                changes: diff::removed(old),
                args: None,
            }
        };
        entries.push(entry);
    }
    (entries, playtime)
}

/// A running batch, see [`Database::begin_batch`]. Finished when dropped at the latest.
pub struct BatchRun {
    log: Option<Arc<ActivityLog>>,
    change: Change,
    counts: BTreeMap<String, u64>,
    finished: bool,
}

impl BatchRun {
    /// The change its entries are recorded with.
    pub fn change(&self) -> &Change {
        &self.change
    }

    /// Adds to one of its counts (`added`, `updated`, ...).
    pub fn count(&mut self, key: &str, n: u64) {
        *self.counts.entry(key.to_string()).or_default() += n;
    }

    /// Ends the batch with its counts.
    pub fn finish(mut self) {
        self.close();
    }

    fn close(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        if let (Some(log), Some(id)) = (&self.log, self.change.batch) {
            log.finish_batch(id, &self.counts);
        }
    }
}

impl Drop for BatchRun {
    fn drop(&mut self) {
        self.close();
    }
}

/// Batches of background work that runs as a queue (the cover and metadata fetchers): the
/// entries of one run of the queue, per actor and reason, share a batch that ends when the
/// queue is empty.
#[derive(Default)]
pub struct RunBatches {
    open: Mutex<HashMap<(Actor, String), i64>>,
}

impl RunBatches {
    /// The change for an entry of the current run of `actor` and `reason` (its batch is started
    /// on first use).
    pub fn change(&self, db: &Database, actor: Actor, reason: &str) -> Change {
        let change = Change::new(actor.clone(), reason);
        let Some(log) = db.activity() else {
            return change;
        };
        let mut open = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        let key = (actor, reason.to_string());
        let id = match open.get(&key) {
            Some(id) => Some(*id),
            None => {
                let id = log.begin_batch(&change);
                if let Some(id) = id {
                    open.insert(key, id);
                }
                id
            }
        };
        change.in_batch(id)
    }

    /// Ends every open batch (the queue ran empty).
    pub fn finish_all(&self, db: &Database) {
        let open: Vec<i64> = self
            .open
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .drain()
            .map(|(_, id)| id)
            .collect();
        if let Some(log) = db.activity() {
            for id in open {
                log.finish_batch(id, &BTreeMap::new());
            }
        }
    }
}
