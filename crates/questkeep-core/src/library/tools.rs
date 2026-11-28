//! An entry's tools: programs and links next to Play, started without tracking.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use uuid::Uuid;

use super::Library;
use crate::activity::{Change, EntryKind, reason};
use crate::error::{CoreError, Result};
use crate::events::CoreEvent;
use crate::model::{ActionKind, GameTool, ImportedToolIcon, PlayAction, ToolIconFile};
use crate::platform;

/// The most tools an entry keeps (the page shows a few, the rest in a menu).
const MAX_TOOLS: usize = 50;
/// Where picked tool icons live, relative to the data dir.
const ICON_PREFIX: &str = "images/tool-icons/";
/// Picture types a tool icon can be (copied as they are); `.exe` files give their icon.
const ICON_EXTENSIONS: [&str; 7] = ["png", "jpg", "jpeg", "webp", "gif", "ico", "svg"];
/// Upper bound for a picked icon picture.
const MAX_ICON_BYTES: u64 = 5 * 1024 * 1024;
/// Size of an icon taken from a program, in pixels.
const PROGRAM_ICON_SIZE: u32 = 48;

/// A stored icon path as the Edit dialog sends it back: only our own files under
/// [`ICON_PREFIX`], never another path.
fn own_icon(icon: Option<&str>) -> Option<String> {
    icon.filter(|icon| {
        icon.strip_prefix(ICON_PREFIX).is_some_and(|name| {
            !name.is_empty() && !name.contains(['/', '\\']) && !name.contains("..")
        })
    })
    .map(str::to_string)
}

/// The host of an https link, in lower case (how site icons are keyed).
fn https_host(url: &str) -> Option<String> {
    let rest = url.trim().strip_prefix("https://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    let host = host.rsplit_once('@').map_or(host, |(_, h)| h);
    let host = host.split(':').next()?;
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

/// A tool's name when the user left it empty: the program's file name or the link's host.
fn default_name(action: &PlayAction) -> String {
    let target = action.target.trim();
    match action.kind {
        // Both separators on every system: a library from a Windows backup keeps its `\` paths.
        ActionKind::File => Path::new(target.rsplit(['/', '\\']).next().unwrap_or(target))
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| target.to_string()),
        ActionKind::Uri => target
            .split_once("://")
            .map(|(_, rest)| rest)
            .unwrap_or(target)
            .split(['/', '?', '#'])
            .next()
            .unwrap_or(target)
            .trim_start_matches("www.")
            .to_string(),
    }
}

fn trimmed(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

/// The checked and tidied form of a tool from the Edit dialog.
fn checked(tool: &GameTool) -> Result<GameTool> {
    let target = tool.action.target.trim();
    if target.is_empty() {
        return Err(CoreError::Invalid(match tool.action.kind {
            ActionKind::File => "Choose the program this tool starts.".into(),
            ActionKind::Uri => "Enter the link this tool opens.".into(),
        }));
    }
    if tool.action.kind == ActionKind::Uri && !has_scheme(target) {
        return Err(CoreError::Invalid(format!(
            "'{target}' isn't a link. Links start with a scheme such as https://."
        )));
    }
    let action = PlayAction {
        kind: tool.action.kind,
        target: target.to_string(),
        args: trimmed(tool.action.args.as_deref()).filter(|_| tool.action.kind == ActionKind::File),
        working_dir: trimmed(tool.action.working_dir.as_deref())
            .filter(|_| tool.action.kind == ActionKind::File),
        ..PlayAction::file("")
    };
    let name = trimmed(Some(&tool.name)).unwrap_or_else(|| default_name(&action));
    Ok(GameTool {
        id: tool.id,
        name,
        action,
        icon: own_icon(tool.icon.as_deref()),
    })
}

/// `scheme:` with a letter first and at least two characters (so `C:\x` is not a link).
fn has_scheme(target: &str) -> bool {
    target.split_once(':').is_some_and(|(scheme, _)| {
        scheme.len() > 1
            && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    })
}

/// A program path as typed; a relative one is taken from the game's install folder.
fn resolve_program(target: &str, install_dir: Option<&str>) -> PathBuf {
    let path = Path::new(target);
    match install_dir {
        Some(dir) if path.is_relative() => Path::new(dir).join(path),
        _ => path.to_path_buf(),
    }
}

impl Library {
    /// The tools of the entry `id` belongs to.
    pub fn game_tools(&self, id: Uuid) -> Result<Vec<GameTool>> {
        let entry = self.db.entry_of(id)?;
        self.db.game_tools(entry)
    }

    /// Saves the Edit dialog's tool list at once (order as given). Nothing is written when one
    /// tool is invalid.
    pub fn replace_game_tools(&self, id: Uuid, tools: &[GameTool]) -> Result<Vec<GameTool>> {
        let entry = self.db.entry_of(id)?;
        if tools.len() > MAX_TOOLS {
            return Err(CoreError::Invalid(format!(
                "A game can have at most {MAX_TOOLS} tools."
            )));
        }
        let tools = tools.iter().map(checked).collect::<Result<Vec<_>>>()?;
        let before = self.db.game_tools(entry)?;
        if before != tools {
            let change = Change::by_caller(reason::TOOLS).kind(EntryKind::Profiles);
            self.db
                .record(&change, &[entry], || self.db.replace_tools(entry, &tools))?;
            self.events
                .publish(CoreEvent::GamesChanged { ids: vec![entry] });
        }
        self.drop_unused_icons(before.iter().chain(&tools).map(|t| t.id), &tools);
        self.db.game_tools(entry)
    }

    /// Deletes the picked icons of these tools that `kept` no longer uses (removed tools, replaced
    /// icons, and icons picked in a dialog that was cancelled).
    fn drop_unused_icons(&self, ids: impl Iterator<Item = Uuid>, kept: &[GameTool]) {
        let ids: Vec<String> = ids.map(|id| id.to_string()).collect();
        let Ok(files) = std::fs::read_dir(self.paths.tool_icons_dir()) else {
            return;
        };
        for file in files.flatten() {
            let name = file.file_name().to_string_lossy().into_owned();
            let ours = ids.iter().any(|id| name.starts_with(id.as_str()));
            let used = kept
                .iter()
                .any(|t| t.icon.as_deref() == Some(&format!("{ICON_PREFIX}{name}")));
            if ours
                && !used
                && file.path().is_file()
                && let Err(err) = std::fs::remove_file(file.path())
            {
                tracing::warn!(%err, file = %name, "could not delete a tool icon");
            }
        }
    }

    /// Copies a picture the user picked as a tool's icon (or takes a program's icon) into the
    /// icon folder and returns its path for the tool, relative to the data dir. The tool keeps
    /// it once the Edit dialog is saved.
    pub fn import_tool_icon(&self, tool: Uuid, source: &str) -> Result<ImportedToolIcon> {
        let dir = self.paths.tool_icons_dir();
        std::fs::create_dir_all(&dir)?;
        let stamp = chrono::Utc::now().timestamp_millis();
        // A picture from the web search: downloaded as it is.
        if source.starts_with("https://") || source.starts_with("http://") {
            let (ext, bytes) = crate::images::download(&crate::images::http_agent(), source)
                .ok_or_else(|| CoreError::Network("The picture could not be downloaded.".into()))?;
            if bytes.len() as u64 > MAX_ICON_BYTES {
                return Err(CoreError::Invalid(
                    "The picture is larger than 5 MB.".into(),
                ));
            }
            let name = format!("{tool}-{stamp}.{ext}");
            std::fs::write(dir.join(&name), bytes)?;
            return Ok(ImportedToolIcon {
                icon: format!("{ICON_PREFIX}{name}"),
                path: dir.join(&name).to_string_lossy().into_owned(),
            });
        }
        let source = Path::new(source);
        let ext = source
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if ext == "exe" {
            let png = platform::executable_icon(source, PROGRAM_ICON_SIZE)
                .and_then(|icon| {
                    crate::images::icon_png_from_rgba(&icon, crate::images::SquareFrom::Icon)
                })
                .ok_or_else(|| CoreError::Invalid("That program has no icon to use.".into()))?;
            let name = format!("{tool}-{stamp}.png");
            std::fs::write(dir.join(&name), png)?;
            return Ok(ImportedToolIcon {
                icon: format!("{ICON_PREFIX}{name}"),
                path: dir.join(&name).to_string_lossy().into_owned(),
            });
        }
        if !ICON_EXTENSIONS.contains(&ext.as_str()) {
            return Err(CoreError::Invalid(
                "Use a PNG, JPG, WebP, GIF, ICO or SVG picture, or a program.".into(),
            ));
        }
        if std::fs::metadata(source)?.len() > MAX_ICON_BYTES {
            return Err(CoreError::Invalid(
                "The picture is larger than 5 MB.".into(),
            ));
        }
        let name = format!("{tool}-{stamp}.{ext}");
        std::fs::copy(source, dir.join(&name))?;
        Ok(ImportedToolIcon {
            icon: format!("{ICON_PREFIX}{name}"),
            path: dir.join(&name).to_string_lossy().into_owned(),
        })
    }

    /// The icon to draw for each of the entry's tools that has one: the picked picture, else the
    /// program's icon (taken once and kept under `tool-icons/auto`), else the link's favicon.
    /// Favicons not fetched yet are fetched in the background; `SiteIconsChanged` follows.
    pub fn tool_icons(&self, id: Uuid) -> Result<Vec<ToolIconFile>> {
        let entry = self.db.entry_of(id)?;
        let tools = self.db.game_tools(entry)?;
        let install_dir = self.db.get_game(entry)?.install_dir;
        let links: Vec<&str> = tools
            .iter()
            .filter(|t| t.icon.is_none() && t.action.kind == ActionKind::Uri)
            .map(|t| t.action.target.as_str())
            .collect();
        if !links.is_empty() {
            self.tool_sites.request(&links);
        }
        let sites = self.tool_sites.icons();
        let mut icons = Vec::new();
        for tool in &tools {
            let path = if let Some(icon) = &tool.icon {
                self.images.resolve(icon)
            } else {
                match tool.action.kind {
                    ActionKind::File => self.program_icon(tool, install_dir.as_deref()),
                    ActionKind::Uri => https_host(&tool.action.target).and_then(|host| {
                        sites
                            .iter()
                            .find(|s| s.host == host)
                            .map(|s| PathBuf::from(&s.path))
                    }),
                }
            };
            if let Some(path) = path {
                icons.push(ToolIconFile {
                    tool_id: tool.id,
                    path: path.to_string_lossy().into_owned(),
                });
            }
        }
        Ok(icons)
    }

    /// The automatic icon a tool of the Edit dialog would get (not saved yet, or changed): the
    /// program's icon, or the link's favicon (fetched in the background when missing;
    /// `SiteIconsChanged` follows). Absolute path, or `None` while there is none.
    pub fn tool_default_icon(&self, id: Uuid, tool: &GameTool) -> Result<Option<String>> {
        let entry = self.db.entry_of(id)?;
        let path = match tool.action.kind {
            ActionKind::File => {
                let install_dir = self.db.get_game(entry)?.install_dir;
                self.program_icon(tool, install_dir.as_deref())
            }
            ActionKind::Uri => {
                let host = https_host(&tool.action.target);
                if host.is_some() {
                    self.tool_sites.request(&[tool.action.target.as_str()]);
                }
                host.and_then(|host| {
                    self.tool_sites
                        .icons()
                        .into_iter()
                        .find(|s| s.host == host)
                        .map(|s| PathBuf::from(s.path))
                })
            }
        };
        Ok(path.map(|p| p.to_string_lossy().into_owned()))
    }

    /// A program tool's icon, taken from its executable once and kept (Windows; elsewhere none).
    fn program_icon(&self, tool: &GameTool, install_dir: Option<&str>) -> Option<PathBuf> {
        use std::hash::{Hash, Hasher};
        let program = resolve_program(&tool.action.target, install_dir);
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        program.hash(&mut hasher);
        let dir = self.paths.tool_icons_dir().join("auto");
        let file = dir.join(format!("{}-{:016x}.png", tool.id, hasher.finish()));
        if file.is_file() {
            return Some(file);
        }
        let png = platform::executable_icon(&program, PROGRAM_ICON_SIZE).and_then(|icon| {
            crate::images::icon_png_from_rgba(&icon, crate::images::SquareFrom::Icon)
        })?;
        std::fs::create_dir_all(&dir).ok()?;
        std::fs::write(&file, png).ok()?;
        Some(file)
    }

    /// Starts a tool: the program (elevated when Windows asks for it) or the link. No tracking,
    /// no scripts, no playtime.
    pub fn run_game_tool(&self, id: Uuid, tool: Uuid) -> Result<()> {
        let entry = self.db.entry_of(id)?;
        let tool = self
            .db
            .game_tools(entry)?
            .into_iter()
            .find(|t| t.id == tool)
            .ok_or_else(|| CoreError::Invalid("That tool is gone.".into()))?;
        let action = &tool.action;
        match action.kind {
            ActionKind::Uri => platform::open_uri(&action.target)
                .map_err(|e| CoreError::Launch(format!("Could not open '{}': {e}", action.target))),
            ActionKind::File => {
                let game = self.db.get_game(entry)?;
                let program = resolve_program(&action.target, game.install_dir.as_deref());
                if !program.is_file() {
                    return Err(CoreError::Launch(format!(
                        "'{}' doesn't exist anymore. Check the tool in Edit > Launch.",
                        program.display()
                    )));
                }
                let dir = action
                    .working_dir
                    .clone()
                    .or_else(|| program.parent().map(|p| p.display().to_string()));
                let mut command = Command::new(&program);
                command
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                if let Some(dir) = &dir {
                    command.current_dir(dir);
                }
                if let Some(args) = action.args.as_deref() {
                    platform::add_raw_args(&mut command, args);
                }
                match command.spawn() {
                    Ok(_) => Ok(()),
                    Err(err) if platform::needs_elevation(&err) => platform::run_elevated(
                        &program.display().to_string(),
                        action.args.as_deref(),
                        dir.as_deref(),
                    )
                    .map_err(|e| {
                        CoreError::Launch(format!("Could not start '{}': {e}", tool.name))
                    }),
                    Err(err) => Err(CoreError::Launch(format!(
                        "Could not start '{}': {err}",
                        tool.name
                    ))),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(kind: ActionKind, name: &str, target: &str) -> GameTool {
        GameTool {
            id: Uuid::nil(),
            name: name.into(),
            action: PlayAction {
                kind,
                ..PlayAction::file(target)
            },
            icon: None,
        }
    }

    #[test]
    fn only_own_icon_paths_are_kept() {
        let mut t = tool(ActionKind::File, "Config", "config.exe");
        t.icon = Some("images/tool-icons/abc-1.png".into());
        assert_eq!(checked(&t).unwrap().icon, t.icon);
        for other in [
            "C:/Users/me/pic.png",
            "images/tool-icons/../library.sqlite",
            "images/tool-icons/",
        ] {
            t.icon = Some(other.into());
            assert_eq!(checked(&t).unwrap().icon, None, "{other}");
        }
    }

    #[test]
    fn https_hosts_are_lower_case_without_port_or_user() {
        assert_eq!(
            https_host("https://User@Wiki.Example.com:8443/a?b").as_deref(),
            Some("wiki.example.com")
        );
        assert_eq!(https_host("http://example.com"), None);
        assert_eq!(https_host("steam://run/1"), None);
    }

    #[test]
    fn empty_names_come_from_the_target() {
        let file = checked(&tool(
            ActionKind::File,
            "  ",
            r"C:\Games\Glimmer\Config.exe",
        ))
        .unwrap();
        assert_eq!(file.name, "Config");
        let link = checked(&tool(
            ActionKind::Uri,
            "",
            "https://www.example.com/wiki/Glimmer?x=1",
        ))
        .unwrap();
        assert_eq!(link.name, "example.com");
        assert_eq!(
            checked(&tool(ActionKind::Uri, " Wiki ", "https://example.com"))
                .unwrap()
                .name,
            "Wiki"
        );
    }

    #[test]
    fn links_need_a_scheme_and_programs_a_path() {
        assert!(checked(&tool(ActionKind::Uri, "x", "example.com")).is_err());
        assert!(checked(&tool(ActionKind::Uri, "x", r"C:\Games\x.exe")).is_err());
        assert!(checked(&tool(ActionKind::Uri, "x", "steam://open/console")).is_ok());
        assert!(checked(&tool(ActionKind::File, "x", "  ")).is_err());
    }

    #[test]
    fn links_drop_program_fields() {
        let mut link = tool(ActionKind::Uri, "Wiki", "https://example.com");
        link.action.args = Some("-x".into());
        link.action.working_dir = Some("C:\\".into());
        let link = checked(&link).unwrap();
        assert_eq!(link.action.args, None);
        assert_eq!(link.action.working_dir, None);
    }

    #[test]
    fn relative_programs_come_from_the_install_folder() {
        assert_eq!(
            resolve_program("tools/config.exe", Some("/games/glimmer")),
            Path::new("/games/glimmer").join("tools/config.exe")
        );
        assert_eq!(resolve_program("config.exe", None), Path::new("config.exe"));
    }
}
