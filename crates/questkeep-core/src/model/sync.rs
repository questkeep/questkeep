use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A store game the user removed and asked not to import again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportExclusion {
    /// The store (provider id).
    pub source: String,
    pub source_game_id: String,
    /// The game's name when it was removed.
    pub name: String,
    pub excluded_at: DateTime<Utc>,
}

/// When stores last finished a sync, kept across restarts (RFC 3339 timestamps).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SyncTimes {
    /// The last "sync all" that finished.
    pub all: Option<String>,
    /// The last successful sync per provider id.
    pub providers: BTreeMap<String, String>,
}
