//! Desktop and Start menu shortcuts that start a game through Questkeep (`--start <id>`).
//! Questkeep records every shortcut it writes, updates it in place when asked again, and deletes
//! it when the game is removed (and, for the ones the "newly installed games" setting made, when
//! it is uninstalled), but only while the file still starts that game.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use super::Library;
use crate::db::ShortcutRecord;
use crate::error::{CoreError, Result};
use crate::model::{ActionKind, Game};
use crate::platform::shortcut::{self, ShortcutIcon, ShortcutPlace, ShortcutSpec};

/// What creating shortcuts did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ShortcutResult {
    /// Shortcuts written (new or updated).
    pub created: u32,
    /// "<game>: <reason>" for each one that failed.
    pub failed: Vec<String>,
}

/// The arguments a shortcut of `entry` passes to Questkeep.
fn start_args(entry: Uuid) -> String {
    format!("--start {entry}")
}

/// The installed entries that are new since `before` was taken: existing entries that became
/// installed, and entries that did not exist yet (only when `count_new` is set: a store's first
/// sync brings every installed game at once, and that is not "newly installed").
pub(crate) fn newly_installed(
    before_installed: &HashSet<Uuid>,
    before_entries: &HashSet<Uuid>,
    after_installed: &HashSet<Uuid>,
    count_new: bool,
) -> Vec<Uuid> {
    let mut ids: Vec<Uuid> = after_installed
        .iter()
        .filter(|id| !before_installed.contains(id))
        .filter(|id| count_new || before_entries.contains(id))
        .copied()
        .collect();
    ids.sort();
    ids
}

impl Library {
    /// The program shortcuts start: this Questkeep executable.
    fn shortcut_target() -> Result<PathBuf> {
        std::env::current_exe()
            .map_err(|e| CoreError::Invalid(format!("Questkeep can't find its own program: {e}")))
    }

    /// The shortcut icon of a game: its own program's icon when it starts one directly, else its
    /// icon image (written as an .ico on Windows), else none (Questkeep's icon).
    fn shortcut_icon(&self, game: &Game) -> ShortcutIcon {
        if cfg!(windows)
            && let Some(action) = &game.play_action
            && action.kind == ActionKind::File
            && Path::new(&action.target).is_file()
        {
            return ShortcutIcon::Resource {
                path: PathBuf::from(&action.target),
                index: 0,
            };
        }
        let Some(png) = game
            .icon_path
            .as_deref()
            .and_then(|icon| self.images.resolve(icon))
        else {
            return ShortcutIcon::None;
        };
        if !cfg!(windows) {
            return ShortcutIcon::Png(png);
        }
        let ico = self
            .paths
            .shortcut_icons_dir()
            .join(format!("{}.ico", game.id));
        let written = std::fs::read(&png)
            .map_err(CoreError::from)
            .and_then(|bytes| shortcut::png_to_ico(&bytes))
            .and_then(|bytes| {
                std::fs::create_dir_all(self.paths.shortcut_icons_dir())?;
                std::fs::write(&ico, bytes)?;
                Ok(())
            });
        match written {
            Ok(()) => ShortcutIcon::Resource {
                path: ico,
                index: 0,
            },
            Err(err) => {
                tracing::warn!(%err, game = %game.id, "could not write the shortcut icon");
                ShortcutIcon::None
            }
        }
    }

    /// Creates (or updates) shortcuts for these games in these places.
    pub fn create_shortcuts(
        &self,
        ids: &[Uuid],
        places: &[ShortcutPlace],
    ) -> Result<ShortcutResult> {
        self.write_shortcuts(ids, places, false)
    }

    fn write_shortcuts(
        &self,
        ids: &[Uuid],
        places: &[ShortcutPlace],
        automatic: bool,
    ) -> Result<ShortcutResult> {
        let target = Self::shortcut_target()?;
        let mut result = ShortcutResult::default();
        let mut entries: Vec<Uuid> = Vec::new();
        for id in ids {
            let entry = self.db.entry_of(*id)?;
            if !entries.contains(&entry) {
                entries.push(entry);
            }
        }
        for entry in entries {
            let game = self.db.get_game(entry)?;
            let args = start_args(entry);
            let existing = self.db.shortcuts_of(&[entry])?;
            let spec = ShortcutSpec {
                name: game.name.clone(),
                target: target.clone(),
                args: args.clone(),
                working_dir: None,
                description: Some(format!("Start {} with Questkeep", game.name)),
                icon: self.shortcut_icon(&game),
            };
            for place in places {
                let Some(dir) = shortcut::place_dir(*place) else {
                    result
                        .failed
                        .push(format!("{}: no folder for shortcuts here", game.name));
                    continue;
                };
                // Update our own shortcut in place, while it still starts this game.
                let ours = existing.iter().find(|r| {
                    r.place == *place && shortcut::points_to(Path::new(&r.path), &target, &args)
                });
                match shortcut::create(&dir, &spec, ours.map(|r| Path::new(&r.path))) {
                    Ok(path) => {
                        let record = ShortcutRecord {
                            path: path.display().to_string(),
                            game_id: entry,
                            place: *place,
                            // A shortcut the user asked for stays when the game is uninstalled.
                            automatic: automatic && ours.is_none_or(|r| r.automatic),
                        };
                        self.db.save_shortcut(&record, Utc::now())?;
                        result.created += 1;
                    }
                    Err(err) => result.failed.push(format!("{}: {err}", game.name)),
                }
            }
        }
        Ok(result)
    }

    /// Deletes the shortcuts Questkeep made for these entries (only the automatic ones with
    /// `only_automatic`). A file that no longer starts the game is left alone; its record goes.
    pub(crate) fn remove_shortcuts(&self, entries: &[Uuid], only_automatic: bool) {
        let records = match self.db.shortcuts_of(entries) {
            Ok(records) => records,
            Err(err) => {
                tracing::warn!(%err, "could not read the recorded shortcuts");
                return;
            }
        };
        if records.is_empty() {
            return;
        }
        let target = Self::shortcut_target().ok();
        for record in records.iter().filter(|r| !only_automatic || r.automatic) {
            let path = Path::new(&record.path);
            let ours = target.as_deref().is_some_and(|target| {
                shortcut::points_to(path, target, &start_args(record.game_id))
            });
            if ours && let Err(err) = std::fs::remove_file(path) {
                tracing::warn!(%err, path = %record.path, "could not delete a shortcut");
                continue;
            }
            if let Err(err) = self.db.delete_shortcut_record(&record.path) {
                tracing::warn!(%err, "could not forget a shortcut");
            }
        }
        for entry in entries {
            if records.iter().all(|r| r.game_id != *entry) {
                continue;
            }
            let ico = self.paths.shortcut_icons_dir().join(format!("{entry}.ico"));
            let left = self.db.shortcuts_of(&[*entry]).map(|r| r.is_empty());
            if left.unwrap_or(false) && ico.is_file() {
                let _ = std::fs::remove_file(ico);
            }
        }
    }

    /// The setting's part of an install change: shortcuts for `installed` entries, the automatic
    /// ones of `uninstalled` entries deleted. Failures are logged; they never fail a sync.
    pub(crate) fn shortcuts_after_installs(&self, installed: &[Uuid], uninstalled: &[Uuid]) {
        if !uninstalled.is_empty() {
            self.remove_shortcuts(uninstalled, true);
        }
        let places = self.settings.load().auto_shortcuts.places();
        if installed.is_empty() || places.is_empty() {
            return;
        }
        let apps: Vec<Uuid> = installed
            .iter()
            .filter(|id| {
                self.db
                    .get_game(**id)
                    .is_ok_and(|g| g.kind == crate::model::GameKind::Game && !g.hidden)
            })
            .copied()
            .collect();
        match self.write_shortcuts(&apps, places, true) {
            Ok(result) => {
                for failure in result.failed {
                    tracing::warn!(failure, "could not create a shortcut for a new install");
                }
            }
            Err(err) => tracing::warn!(%err, "could not create shortcuts for new installs"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newly_installed_counts_new_entries_only_when_asked() {
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        let c = Uuid::from_u128(3);
        let before_installed = HashSet::from([a]);
        let before_entries = HashSet::from([a, b]);
        let after = HashSet::from([a, b, c]);
        assert_eq!(
            newly_installed(&before_installed, &before_entries, &after, false),
            vec![b]
        );
        let mut both = vec![b, c];
        both.sort();
        assert_eq!(
            newly_installed(&before_installed, &before_entries, &after, true),
            both
        );
    }
}
