//! An entry's tools (see [`crate::model::GameTool`]).

use rusqlite::params;
use uuid::Uuid;

use super::Database;
use crate::error::Result;
use crate::model::GameTool;

impl Database {
    /// The tools of an entry in the user's order. A malformed row is skipped.
    pub fn game_tools(&self, entry_id: Uuid) -> Result<Vec<GameTool>> {
        let conn = self.conn();
        let mut stmt = conn.prepare_cached(
            "SELECT id, name, action, icon FROM game_tools WHERE entry_id = ?1 ORDER BY position, rowid",
        )?;
        let rows = stmt
            .query_map([entry_id.to_string()], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows
            .into_iter()
            .filter_map(|(id, name, action, icon)| {
                Some(GameTool {
                    id: Uuid::parse_str(&id).ok()?,
                    name,
                    action: serde_json::from_str(&action).ok()?,
                    icon,
                })
            })
            .collect())
    }

    /// Replaces an entry's tools with `tools`, in this order, all or nothing.
    pub fn replace_tools(&self, entry_id: Uuid, tools: &[GameTool]) -> Result<()> {
        self.check_recorded("replace_tools");
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM game_tools WHERE entry_id = ?1",
            [entry_id.to_string()],
        )?;
        for (position, tool) in tools.iter().enumerate() {
            tx.execute(
                "INSERT INTO game_tools (id, entry_id, name, action, position, icon) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    tool.id.to_string(),
                    entry_id.to_string(),
                    tool.name,
                    serde_json::to_string(&tool.action)?,
                    position as i64,
                    tool.icon
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PlayAction;
    use crate::model::{GameQuery, ImportedGame};

    #[test]
    fn tools_are_replaced_in_order() {
        let db = Database::open_in_memory().unwrap();
        db.upsert_imported(
            "steam",
            &[ImportedGame::installed("1", "Glimmer")],
            chrono::Utc::now(),
        )
        .unwrap();
        let game = db.list_games(&GameQuery::default()).unwrap()[0].id;
        let config = GameTool {
            id: Uuid::new_v4(),
            name: "Config".into(),
            action: PlayAction::file(r"C:\Games\Glimmer\config.exe"),
            icon: Some("images/tool-icons/config.png".into()),
        };
        let wiki = GameTool {
            id: Uuid::new_v4(),
            name: "Wiki".into(),
            action: PlayAction::uri("https://example.com/wiki"),
            icon: None,
        };
        db.replace_tools(game, &[wiki.clone(), config.clone()])
            .unwrap();
        assert_eq!(db.game_tools(game).unwrap(), vec![wiki.clone(), config]);
        db.replace_tools(game, std::slice::from_ref(&wiki)).unwrap();
        assert_eq!(db.game_tools(game).unwrap(), vec![wiki]);
    }
}
