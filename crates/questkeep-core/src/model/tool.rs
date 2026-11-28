use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use super::PlayAction;

/// A program or link next to Play on an entry's page (a config tool, a mod manager, a wiki).
/// Started without tracking, scripts or playtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameTool {
    pub id: Uuid,
    pub name: String,
    /// `file` (a program, with arguments and working folder) or `uri` (a link).
    pub action: PlayAction,
    /// An icon the user picked, relative to the data dir (`images/tool-icons/...`); without one
    /// the page shows the program's icon or the site's favicon.
    #[serde(default)]
    pub icon: Option<String>,
}

/// A picture picked as a tool's icon, copied into the icon folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportedToolIcon {
    /// What the tool keeps (`GameTool::icon`), relative to the data dir.
    pub icon: String,
    /// Absolute path of the copy, for the Edit dialog's preview.
    pub path: String,
}

/// The icon file to draw for a tool on the game page (picked, the program's, or the favicon).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ToolIconFile {
    pub tool_id: Uuid,
    /// Absolute path of the picture.
    pub path: String,
}
