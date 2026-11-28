//! The desktop and Start menu shortcuts Questkeep made, so it can update and remove them. Not
//! library data: the activity log does not record these rows.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use rusqlite::params;
use uuid::Uuid;

use super::Database;
use crate::error::Result;
use crate::platform::shortcut::ShortcutPlace;

/// A shortcut file Questkeep wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutRecord {
    pub path: String,
    /// The entry it starts.
    pub game_id: Uuid,
    pub place: ShortcutPlace,
    /// Made by the "newly installed games" setting (removed again on uninstall).
    pub automatic: bool,
}

impl Database {
    /// The recorded shortcuts of these entries.
    pub fn shortcuts_of(&self, game_ids: &[Uuid]) -> Result<Vec<ShortcutRecord>> {
        let conn = self.conn();
        let mut stmt = conn.prepare_cached(
            "SELECT path, game_id, place, automatic FROM game_shortcuts WHERE game_id = ?1 \
             ORDER BY created_at",
        )?;
        let mut out = Vec::new();
        for id in game_ids {
            let rows = stmt
                .query_map([id.to_string()], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, bool>(3)?,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            out.extend(
                rows.into_iter()
                    .filter_map(|(path, game, place, automatic)| {
                        Some(ShortcutRecord {
                            path,
                            game_id: Uuid::parse_str(&game).ok()?,
                            place: ShortcutPlace::parse(&place)?,
                            automatic,
                        })
                    }),
            );
        }
        Ok(out)
    }

    /// Records a shortcut (replacing a record of the same file).
    pub fn save_shortcut(&self, record: &ShortcutRecord, now: DateTime<Utc>) -> Result<()> {
        self.conn().execute(
            "INSERT INTO game_shortcuts (path, game_id, place, automatic, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT (path) DO UPDATE SET \
             game_id = excluded.game_id, place = excluded.place, automatic = excluded.automatic",
            params![
                record.path,
                record.game_id.to_string(),
                record.place.as_str(),
                record.automatic,
                now
            ],
        )?;
        Ok(())
    }

    pub fn delete_shortcut_record(&self, path: &str) -> Result<()> {
        self.conn()
            .execute("DELETE FROM game_shortcuts WHERE path = ?1", [path])?;
        Ok(())
    }

    /// The entries with an installed copy (or that are installed themselves).
    pub fn installed_entry_ids(&self) -> Result<HashSet<Uuid>> {
        let conn = self.conn();
        let mut stmt = conn.prepare_cached(
            "SELECT DISTINCT COALESCE(merged_into, id) FROM games WHERE installed = 1",
        )?;
        let ids = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(ids
            .into_iter()
            .filter_map(|id| Uuid::parse_str(&id).ok())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{GameQuery, ImportedGame};

    #[test]
    fn records_and_installed_entries() {
        let db = Database::open_in_memory().unwrap();
        db.upsert_imported(
            "steam",
            &[
                ImportedGame::installed("1", "Alpha"),
                ImportedGame {
                    installed: false,
                    ..ImportedGame::installed("2", "Beta")
                },
            ],
            Utc::now(),
        )
        .unwrap();
        let games = db.list_games(&GameQuery::default()).unwrap();
        let alpha = games.iter().find(|g| g.name == "Alpha").unwrap().id;
        let beta = games.iter().find(|g| g.name == "Beta").unwrap().id;
        assert_eq!(db.installed_entry_ids().unwrap(), HashSet::from([alpha]));
        assert_eq!(
            db.entry_ids().unwrap().into_iter().collect::<HashSet<_>>(),
            HashSet::from([alpha, beta])
        );

        let record = ShortcutRecord {
            path: r"C:\Users\x\Desktop\Alpha.lnk".into(),
            game_id: alpha,
            place: ShortcutPlace::Desktop,
            automatic: true,
        };
        db.save_shortcut(&record, Utc::now()).unwrap();
        assert_eq!(
            db.shortcuts_of(&[alpha, beta]).unwrap(),
            vec![record.clone()]
        );
        db.delete_shortcut_record(&record.path).unwrap();
        assert!(db.shortcuts_of(&[alpha]).unwrap().is_empty());
    }
}
