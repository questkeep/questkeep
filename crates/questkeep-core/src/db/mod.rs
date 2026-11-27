//! SQLite persistence: connection handling, migrations and repositories.

mod accounts;
mod backup;
mod bulk;
mod categories;
mod combine;
mod games;
mod metadata;
pub mod migrations;
mod playtime;
pub use migrations::LATEST_VERSION;
mod profiles;
mod scripts;
mod sessions;
mod setup;
mod start;
mod statistics;
pub mod text_match;
mod views;

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use rusqlite::Connection;

use crate::error::{CoreError, Result};

pub use accounts::KnownAccount;
pub use combine::{Combined, duplicate_key};
pub use games::{DetectionDir, GameMissingMetadata, InstallTarget, NewGameRecord, StateRow};
pub use playtime::{StatSession, StoreDay};
pub use profiles::ProfileRow;
pub use sessions::OpenSession;
pub use start::{SessionSpan, playtime_by_day, playtime_by_week};
pub use statistics::{StatCopy, StatEntry};

/// The library database. One connection guarded by a mutex: SQLite serializes writes anyway and
/// every query here takes well under a millisecond for realistic library sizes.
#[derive(Debug)]
pub struct Database {
    conn: Mutex<Connection>,
    /// The activity log the recorder writes to (see [crate::activity]); unset in tools and in
    /// tests that use the database alone.
    activity: OnceLock<Arc<crate::activity::ActivityLog>>,
}

impl Database {
    /// Opens (or creates) the database file and applies pending migrations. An existing database
    /// with an older schema is backed up first; if that fails, nothing is migrated.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        let version = migrations::current_version(&conn)?;
        if version > 0 && version < migrations::LATEST_VERSION {
            let backup = backup::backup_before_migration(&conn, path, version)?;
            tracing::info!(version, backup = %backup.display(), "backed up database before migrating");
        }
        Self::init(conn)
    }

    /// In-memory database for tests and tooling.
    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        text_match::register(&conn)?;
        migrations::migrate(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            activity: OnceLock::new(),
        })
    }

    /// Writes a consistent copy of the database (WAL content included) to `path`, which must
    /// not exist yet.
    pub fn copy_to(&self, path: &std::path::Path) -> Result<()> {
        let target = path.to_str().ok_or_else(|| {
            CoreError::Invalid(format!("{} is not a valid path.", path.display()))
        })?;
        self.conn().execute("VACUUM INTO ?1", [target])?;
        Ok(())
    }

    /// Makes the recorder write to `log` (once; later calls are ignored). From then on, debug
    /// builds check that game-mutating functions run inside a recorded change.
    pub fn attach_activity(&self, log: Arc<crate::activity::ActivityLog>) {
        let _ = self.activity.set(log);
    }

    /// The activity log, when one is attached.
    pub fn activity(&self) -> Option<&Arc<crate::activity::ActivityLog>> {
        self.activity.get()
    }

    /// Called first by every function that changes what the activity log shows of a game: with
    /// an activity log attached, it must run inside a recorded change ([`Self::record`] and
    /// friends). A forgotten write site fails the unit tests and is logged as an error in other
    /// debug builds; release builds skip the check. Functions that only touch internal state
    /// (markers, heartbeats, `state_version`) don't call it.
    #[inline]
    pub(crate) fn check_recorded(&self, what: &str) {
        if cfg!(debug_assertions) && self.activity.get().is_some() && !crate::activity::in_scope() {
            if cfg!(test) {
                panic!("{what} changed the library outside a recorded change (activity log)");
            }
            tracing::error!(
                what,
                "a library change was not recorded in the activity log"
            );
        }
    }

    /// Locks the connection. A poisoned lock is recovered: SQLite rolls back any transaction that
    /// was open when the panicking thread dropped it, so the data stays consistent.
    pub(crate) fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
