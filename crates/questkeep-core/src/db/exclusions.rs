//! Removing entries, and the store games a removal keeps out of later syncs.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;

use super::Database;
use crate::error::{CoreError, Result};
use crate::model::{ImportExclusion, MANUAL_SOURCE};

impl Database {
    /// Deletes several entries with all their copies, all or nothing (an unknown id deletes
    /// nothing). With `exclude`, every store copy is remembered so syncs no longer import it.
    /// Returns the ids of the deleted copies (the entries' own ids left out).
    pub fn delete_games(
        &self,
        ids: &[Uuid],
        exclude: bool,
        now: DateTime<Utc>,
    ) -> Result<Vec<Uuid>> {
        self.check_recorded("delete_games");
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let mut copies = Vec::new();
        for id in ids {
            let key = id.to_string();
            let rows: Vec<(String, String, Option<String>, String)> = {
                let mut stmt = tx.prepare_cached(
                    "SELECT id, source, source_game_id, name FROM games \
                     WHERE id = ?1 OR merged_into = ?1",
                )?;
                stmt.query_map([&key], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
                    .collect::<rusqlite::Result<_>>()?
            };
            if !rows.iter().any(|(row, ..)| *row == key) {
                return Err(CoreError::GameNotFound(*id));
            }
            for (row, source, source_game_id, name) in &rows {
                if exclude
                    && source != MANUAL_SOURCE
                    && let Some(source_game_id) = source_game_id
                {
                    tx.execute(
                        "INSERT OR REPLACE INTO import_exclusions \
                         (source, source_game_id, name, excluded_at) VALUES (?1, ?2, ?3, ?4)",
                        params![source, source_game_id, name, now],
                    )?;
                }
                if *row != key
                    && let Ok(copy) = Uuid::parse_str(row)
                {
                    copies.push(copy);
                }
            }
            tx.execute("DELETE FROM games WHERE merged_into = ?1", [&key])?;
            tx.execute("DELETE FROM games WHERE id = ?1", [&key])?;
        }
        tx.commit()?;
        Ok(copies)
    }

    /// Every excluded store game, most recently excluded first.
    pub fn list_import_exclusions(&self) -> Result<Vec<ImportExclusion>> {
        let conn = self.conn();
        let mut stmt = conn.prepare_cached(
            "SELECT source, source_game_id, name, excluded_at FROM import_exclusions \
             ORDER BY excluded_at DESC, name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(ImportExclusion {
                source: r.get(0)?,
                source_game_id: r.get(1)?,
                name: r.get(2)?,
                excluded_at: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Lets the next sync import the game again; returns the removed exclusion, `None` when
    /// there was none.
    pub fn remove_import_exclusion(
        &self,
        source: &str,
        source_game_id: &str,
    ) -> Result<Option<ImportExclusion>> {
        let conn = self.conn();
        let found = conn
            .query_row(
                "DELETE FROM import_exclusions WHERE source = ?1 AND source_game_id = ?2 \
                 RETURNING source, source_game_id, name, excluded_at",
                params![source, source_game_id],
                |r| {
                    Ok(ImportExclusion {
                        source: r.get(0)?,
                        source_game_id: r.get(1)?,
                        name: r.get(2)?,
                        excluded_at: r.get(3)?,
                    })
                },
            )
            .optional()?;
        Ok(found)
    }
}

/// The store game ids of `source` that syncs must not import.
pub(super) fn excluded_ids(conn: &Connection, source: &str) -> Result<HashSet<String>> {
    let mut stmt =
        conn.prepare_cached("SELECT source_game_id FROM import_exclusions WHERE source = ?1")?;
    let rows = stmt.query_map([source], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::model::{GameQuery, HiddenFilter, ImportedGame};

    fn names(db: &Database) -> Vec<String> {
        let query = GameQuery {
            hidden: HiddenFilter::All,
            ..GameQuery::default()
        };
        let mut list: Vec<String> = db
            .list_games(&query)
            .unwrap()
            .into_iter()
            .map(|g| g.name)
            .collect();
        list.sort();
        list
    }

    fn id_of(db: &Database, name: &str) -> Uuid {
        db.list_games(&GameQuery::default())
            .unwrap()
            .into_iter()
            .find(|g| g.name == name)
            .unwrap()
            .id
    }

    fn sync(db: &Database, source: &str, games: &[ImportedGame]) {
        db.upsert_imported(source, games, Utc::now()).unwrap();
    }

    #[test]
    fn excluded_games_are_not_imported_again() {
        let db = Database::open_in_memory().unwrap();
        let scan = [
            ImportedGame::installed("1", "Alpha"),
            ImportedGame::installed("2", "Beta"),
        ];
        sync(&db, "steam", &scan);
        db.delete_games(&[id_of(&db, "Alpha")], true, Utc::now())
            .unwrap();

        sync(&db, "steam", &scan);
        assert_eq!(names(&db), vec!["Beta"]);
        let list = db.list_import_exclusions().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(
            (
                list[0].source.as_str(),
                list[0].source_game_id.as_str(),
                list[0].name.as_str()
            ),
            ("steam", "1", "Alpha")
        );
        // The same id in another store is a different game.
        sync(&db, "epic", &[ImportedGame::installed("1", "Alpha (Epic)")]);
        assert_eq!(names(&db), vec!["Alpha (Epic)", "Beta"]);

        // Bringing it back: the next sync imports it as a new game.
        let removed = db.remove_import_exclusion("steam", "1").unwrap();
        assert_eq!(removed.map(|e| e.name), Some("Alpha".to_string()));
        assert_eq!(db.remove_import_exclusion("steam", "1").unwrap(), None);
        sync(&db, "steam", &scan);
        assert_eq!(names(&db), vec!["Alpha", "Alpha (Epic)", "Beta"]);
    }

    #[test]
    fn removing_without_exclude_lets_the_game_come_back() {
        let db = Database::open_in_memory().unwrap();
        let scan = [ImportedGame::installed("1", "Alpha")];
        sync(&db, "steam", &scan);
        db.delete_games(&[id_of(&db, "Alpha")], false, Utc::now())
            .unwrap();
        assert!(names(&db).is_empty());
        sync(&db, "steam", &scan);
        assert_eq!(names(&db), vec!["Alpha"]);
        assert!(db.list_import_exclusions().unwrap().is_empty());
    }

    #[test]
    fn removing_a_combined_entry_removes_and_excludes_every_copy() {
        let db = Database::open_in_memory().unwrap();
        sync(&db, "steam", &[ImportedGame::installed("10", "Gamma")]);
        sync(&db, "gog", &[ImportedGame::installed("20", "Gamma GOG")]);
        let steam = id_of(&db, "Gamma");
        let gog = id_of(&db, "Gamma GOG");
        db.combine_games(steam, &[gog]).unwrap();

        let copies = db.delete_games(&[steam], true, Utc::now()).unwrap();
        assert_eq!(copies, vec![gog]);
        assert!(names(&db).is_empty());

        sync(&db, "steam", &[ImportedGame::installed("10", "Gamma")]);
        sync(&db, "gog", &[ImportedGame::installed("20", "Gamma GOG")]);
        assert!(names(&db).is_empty());
        assert_eq!(db.list_import_exclusions().unwrap().len(), 2);
    }

    #[test]
    fn manual_games_are_never_excluded() {
        let db = Database::open_in_memory().unwrap();
        let game = db
            .insert_game(
                &super::super::NewGameRecord {
                    name: "Homebrew".into(),
                    source: MANUAL_SOURCE.into(),
                    source_game_id: Some("x".into()),
                    ..Default::default()
                },
                Utc::now(),
            )
            .unwrap();
        db.delete_games(&[game.id], true, Utc::now()).unwrap();
        assert!(db.list_import_exclusions().unwrap().is_empty());
    }

    #[test]
    fn an_unknown_id_deletes_nothing() {
        let db = Database::open_in_memory().unwrap();
        sync(&db, "steam", &[ImportedGame::installed("1", "Alpha")]);
        let ids = [id_of(&db, "Alpha"), Uuid::new_v4()];
        assert!(db.delete_games(&ids, true, Utc::now()).is_err());
        assert_eq!(names(&db), vec!["Alpha"]);
        assert!(db.list_import_exclusions().unwrap().is_empty());
    }
}
