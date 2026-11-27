//! Schema of `activity.sqlite`. Like the library's: never edit a released step, append one.

use rusqlite::{Connection, OptionalExtension};

use crate::error::{CoreError, Result};

/// Migration steps; index + 1 is the schema version the step produces. Times are milliseconds
/// since the Unix epoch (they sort and page cheaply).
const MIGRATIONS: &[&str] = &[
    // 1: entries (one per game per action), their field changes, batches and system events.
    r#"
    CREATE TABLE batches (
        id          INTEGER PRIMARY KEY,
        started_at  INTEGER NOT NULL,
        finished_at INTEGER,
        actor_kind  TEXT NOT NULL,
        actor_id    TEXT,
        reason      TEXT NOT NULL,
        args_json   TEXT,
        counts_json TEXT
    );
    CREATE INDEX batches_started ON batches (started_at);

    CREATE TABLE entries (
        id           INTEGER PRIMARY KEY,
        at           INTEGER NOT NULL,
        batch_id     INTEGER REFERENCES batches (id) ON DELETE CASCADE,
        actor_kind   TEXT NOT NULL,
        actor_id     TEXT,
        reason       TEXT NOT NULL,
        args_json    TEXT,
        game_id      TEXT,
        game_name    TEXT NOT NULL,
        kind         TEXT NOT NULL,
        change_count INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX entries_at ON entries (at);
    CREATE INDEX entries_game ON entries (game_id, at);
    CREATE INDEX entries_batch ON entries (batch_id);
    CREATE INDEX entries_actor ON entries (actor_kind, actor_id, at);

    CREATE TABLE changes (
        entry_id    INTEGER NOT NULL REFERENCES entries (id) ON DELETE CASCADE,
        position    INTEGER NOT NULL,
        field       TEXT NOT NULL,
        mode        TEXT NOT NULL,
        before_json TEXT,
        after_json  TEXT,
        PRIMARY KEY (entry_id, position)
    ) WITHOUT ROWID;

    CREATE TABLE system_events (
        id           INTEGER PRIMARY KEY,
        at           INTEGER NOT NULL,
        level        TEXT NOT NULL,
        source_kind  TEXT NOT NULL,
        source_id    TEXT,
        kind         TEXT NOT NULL,
        message      TEXT NOT NULL,
        details_json TEXT
    );
    CREATE INDEX system_events_at ON system_events (at);
    "#,
];

/// Latest schema version of the activity database.
pub const LATEST_VERSION: u32 = MIGRATIONS.len() as u32;

fn current_version(conn: &Connection) -> Result<u32> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);")?;
    let version: Option<u32> = conn
        .query_row("SELECT version FROM schema_version LIMIT 1", [], |r| {
            r.get(0)
        })
        .optional()?;
    Ok(version.unwrap_or(0))
}

/// Applies the pending steps, each in its own transaction.
pub fn migrate(conn: &mut Connection) -> Result<u32> {
    let mut version = current_version(conn)?;
    if version > LATEST_VERSION {
        return Err(CoreError::Invalid(format!(
            "The activity log was written by a newer Questkeep (schema {version}, this build \
             supports {LATEST_VERSION})."
        )));
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        let target = index as u32 + 1;
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.execute("DELETE FROM schema_version", [])?;
        tx.execute("INSERT INTO schema_version (version) VALUES (?1)", [target])?;
        tx.commit()?;
        version = target;
    }
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_a_fresh_database_once() {
        let mut conn = Connection::open_in_memory().unwrap();
        assert_eq!(migrate(&mut conn).unwrap(), LATEST_VERSION);
        assert_eq!(migrate(&mut conn).unwrap(), LATEST_VERSION);
        conn.execute("DELETE FROM schema_version", []).unwrap();
        conn.execute("INSERT INTO schema_version VALUES (99)", [])
            .unwrap();
        assert!(migrate(&mut conn).is_err());
    }
}
