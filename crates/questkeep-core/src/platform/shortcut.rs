//! Shortcuts that start a game through Questkeep: `.lnk` files on Windows (Desktop, Start menu),
//! `.desktop` files on Linux. The caller decides names, folders and icons; this module only writes,
//! recognizes and names the files. COM is the one `unsafe` part (see `docs/DECISIONS.md`).
#![allow(unsafe_code)]

use std::path::{Path, PathBuf};

use crate::error::{CoreError, Result};

/// The icon of a shortcut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShortcutIcon {
    /// No icon of its own (the target's icon is used).
    None,
    /// An .ico/.exe/.dll and the icon index in it.
    Resource {
        /// The file with the icon.
        path: PathBuf,
        /// The icon's index in that file.
        index: i32,
    },
    /// A PNG file (Linux uses it directly; Windows needs `Resource` with an .ico).
    Png(PathBuf),
}

/// What a shortcut starts and how it looks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutSpec {
    /// The shown name, also the file stem.
    pub name: String,
    /// The program to start.
    pub target: PathBuf,
    /// Its command line arguments.
    pub args: String,
    /// The working directory, if not the target's folder.
    pub working_dir: Option<PathBuf>,
    /// A tooltip/comment.
    pub description: Option<String>,
    /// The icon.
    pub icon: ShortcutIcon,
}

/// Where shortcuts go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ShortcutPlace {
    /// The user's desktop.
    Desktop,
    /// The Start menu (Windows: a "Questkeep Games" folder in Programs) or the applications menu
    /// (Linux).
    StartMenu,
}

impl ShortcutPlace {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::StartMenu => "startMenu",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "desktop" => Some(Self::Desktop),
            "startMenu" => Some(Self::StartMenu),
            _ => None,
        }
    }
}

/// The folder for a place. Windows: Desktop = `FOLDERID_Desktop`, StartMenu = `FOLDERID_Programs`
/// joined with "Questkeep Games". Linux: Desktop = `XDG_DESKTOP_DIR` from
/// `~/.config/user-dirs.dirs` (else `~/Desktop`), StartMenu = `$XDG_DATA_HOME/applications`
/// (default `~/.local/share/applications`). Other systems: `None`.
pub fn place_dir(place: ShortcutPlace) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        win::place_dir(place)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        linux_place_dir(place)
    }
    #[cfg(not(any(windows, all(unix, not(target_os = "macos")))))]
    {
        let _ = place;
        None
    }
}

const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// A file name for `name` that is valid on Windows and Linux: strips `<>:"/\|?*` and control
/// characters and trailing dots/spaces, gives reserved device names (CON, NUL, COM1...) a suffix,
/// turns an empty name into "Game" and keeps at most 100 characters. The extension is ".lnk" on
/// Windows and ".desktop" on Linux (Linux files are prefixed "questkeep-" so menus don't clash).
pub fn file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| !c.is_control() && !"<>:\"/\\|?*".contains(*c))
        .collect();
    let mut stem: String = cleaned.chars().take(100).collect();
    stem = stem.trim_matches([' ', '.']).to_string();
    if stem.is_empty() {
        stem = "Game".to_string();
    }
    let device = stem.split('.').next().unwrap_or("").trim_end();
    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(device)) {
        stem.push_str(" (game)");
    }
    if cfg!(windows) {
        format!("{stem}.lnk")
    } else {
        format!("questkeep-{stem}.desktop")
    }
}

/// Writes the shortcut into `dir` (created if missing) and returns the written path. If a file
/// with that name exists, " (2)", " (3)"... is appended, unless it is `replace`, an existing
/// shortcut of ours (the caller tracks which files it made), which is overwritten in place.
pub fn create(dir: &Path, spec: &ShortcutSpec, replace: Option<&Path>) -> Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = match replace {
        Some(existing) if existing.is_file() => existing.to_path_buf(),
        _ => free_path(dir, &file_name(&spec.name)),
    };
    write(&path, spec)?;
    tracing::info!(path = %path.display(), "created shortcut");
    Ok(path)
}

/// The first path in `dir` for `file` that does not exist yet.
fn free_path(dir: &Path, file: &str) -> PathBuf {
    let first = dir.join(file);
    if !first.exists() {
        return first;
    }
    let (stem, ext) = match file.rfind('.') {
        Some(dot) => file.split_at(dot),
        None => (file, ""),
    };
    (2..)
        .map(|n| dir.join(format!("{stem} ({n}){ext}")))
        .find(|p| !p.exists())
        .unwrap_or(first)
}

fn write(path: &Path, spec: &ShortcutSpec) -> Result<()> {
    #[cfg(windows)]
    {
        win::write(path, spec)
    }
    #[cfg(not(windows))]
    {
        linux_write(path, spec)
    }
}

/// Whether `path` is still a shortcut to `target` with these `args`, so the caller deletes only
/// what it made and the user did not change.
pub fn points_to(path: &Path, target: &Path, args: &str) -> bool {
    #[cfg(windows)]
    {
        win::points_to(path, target, args)
    }
    #[cfg(not(windows))]
    {
        linux_points_to(path, target, args)
    }
}

/// Wraps PNG bytes in a single-image .ico (a PNG-compressed entry, Vista and later), reading
/// width and height from the IHDR; 256 is stored as 0.
pub fn png_to_ico(png: &[u8]) -> Result<Vec<u8>> {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let invalid = || CoreError::Invalid("The icon is not a valid PNG image.".into());
    if png.len() < 33 || png[..8] != SIGNATURE || &png[12..16] != b"IHDR" {
        return Err(invalid());
    }
    let dim = |at: usize| u32::from_be_bytes([png[at], png[at + 1], png[at + 2], png[at + 3]]);
    let (w, h) = (dim(16), dim(20));
    if w == 0 || h == 0 || w > 256 || h > 256 {
        return Err(CoreError::Invalid(
            "The icon must be between 1 and 256 pixels wide and high.".into(),
        ));
    }
    let size = u32::try_from(png.len()).map_err(|_| invalid())?;
    let mut ico = Vec::with_capacity(22 + png.len());
    ico.extend_from_slice(&[0, 0, 1, 0, 1, 0]); // ICONDIR: reserved, type icon, one image
    ico.push((w % 256) as u8);
    ico.push((h % 256) as u8);
    ico.extend_from_slice(&[0, 0]); // palette size, reserved
    ico.extend_from_slice(&1u16.to_le_bytes()); // planes
    ico.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
    ico.extend_from_slice(&size.to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes()); // offset of the image data
    ico.extend_from_slice(png);
    Ok(ico)
}

// ---- .desktop files (written on Linux; pure so they are tested everywhere) ----

/// The value of a `.desktop` string key: backslash, newline and tab escaped.
#[cfg(any(not(windows), test))]
fn desktop_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' | '\r' => out.push(' '),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

/// The `Exec` value: the quoted target (`"`, backtick, `$` and `\` escaped for the Exec quoting,
/// `%` doubled so it is no field code, then the string escaping of the key) and the arguments.
#[cfg(any(not(windows), test))]
fn desktop_exec(target: &Path, args: &str) -> String {
    let mut quoted = String::from("\"");
    for c in target.to_string_lossy().chars() {
        match c {
            '"' | '`' | '$' | '\\' => {
                quoted.push('\\');
                quoted.push(c);
            }
            '%' => quoted.push_str("%%"),
            c => quoted.push(c),
        }
    }
    quoted.push('"');
    let line = if args.trim().is_empty() {
        quoted
    } else {
        format!("{quoted} {}", args.trim())
    };
    desktop_string(&line)
}

/// The text of the `.desktop` file for a shortcut.
#[cfg(any(not(windows), test))]
fn desktop_entry(spec: &ShortcutSpec) -> String {
    let mut text = String::from("[Desktop Entry]\nType=Application\n");
    text.push_str(&format!("Name={}\n", desktop_string(&spec.name)));
    if let Some(description) = spec.description.as_deref().filter(|d| !d.is_empty()) {
        text.push_str(&format!("Comment={}\n", desktop_string(description)));
    }
    text.push_str(&format!(
        "Exec={}\n",
        desktop_exec(&spec.target, &spec.args)
    ));
    if let Some(dir) = &spec.working_dir {
        text.push_str(&format!(
            "Path={}\n",
            desktop_string(&dir.to_string_lossy())
        ));
    }
    match &spec.icon {
        ShortcutIcon::Png(path) | ShortcutIcon::Resource { path, .. } => {
            text.push_str(&format!(
                "Icon={}\n",
                desktop_string(&path.to_string_lossy())
            ));
        }
        ShortcutIcon::None => {}
    }
    text.push_str("Terminal=false\nCategories=Game;\n");
    text
}

#[cfg(any(not(windows), test))]
fn desktop_points_to(content: &str, target: &Path, args: &str) -> bool {
    let expected = format!("Exec={}", desktop_exec(target, args));
    content.lines().any(|line| line == expected)
}

#[cfg(not(windows))]
fn linux_write(path: &Path, spec: &ShortcutSpec) -> Result<()> {
    std::fs::write(path, desktop_entry(spec))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

#[cfg(not(windows))]
fn linux_points_to(path: &Path, target: &Path, args: &str) -> bool {
    std::fs::read_to_string(path).is_ok_and(|text| desktop_points_to(&text, target, args))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn linux_place_dir(place: ShortcutPlace) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    match place {
        ShortcutPlace::Desktop => {
            let config = std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .unwrap_or_else(|| home.join(".config"));
            let from_file = std::fs::read_to_string(config.join("user-dirs.dirs"))
                .ok()
                .and_then(|text| parse_desktop_dir(&text, &home));
            Some(from_file.unwrap_or_else(|| home.join("Desktop")))
        }
        ShortcutPlace::StartMenu => {
            let data = std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .unwrap_or_else(|| home.join(".local/share"));
            Some(data.join("applications"))
        }
    }
}

/// `XDG_DESKTOP_DIR="$HOME/Desktop"` from a user-dirs.dirs file.
#[cfg(all(unix, not(target_os = "macos")))]
fn parse_desktop_dir(text: &str, home: &Path) -> Option<PathBuf> {
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("XDG_DESKTOP_DIR="))?;
    let value = line.split_once('=')?.1.trim().trim_matches('"');
    let path = match value.strip_prefix("$HOME") {
        Some(rest) => home.join(rest.trim_start_matches('/')),
        None => PathBuf::from(value),
    };
    (path.is_absolute() && !value.is_empty()).then_some(path)
}

// ---- Windows: IShellLinkW ----

#[cfg(windows)]
mod win {
    use std::path::{Path, PathBuf};

    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        CoTaskMemFree, CoUninitialize, IPersistFile, STGM_READ,
    };
    use windows::Win32::UI::Shell::{
        FOLDERID_Desktop, FOLDERID_Programs, IShellLinkW, KF_FLAG_DEFAULT, SHGetKnownFolderPath,
        ShellLink,
    };
    use windows::core::{HSTRING, Interface};

    use super::{ShortcutIcon, ShortcutPlace, ShortcutSpec};
    use crate::error::{CoreError, Result};

    /// Balances a successful `CoInitializeEx` with `CoUninitialize`; a thread that already has
    /// COM in another mode (RPC_E_CHANGED_MODE) is used as it is.
    struct Com(bool);

    impl Com {
        fn init() -> Self {
            // S_OK and S_FALSE both need a matching CoUninitialize; an error does not.
            Self(unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok())
        }
    }

    impl Drop for Com {
        fn drop(&mut self) {
            if self.0 {
                unsafe { CoUninitialize() };
            }
        }
    }

    pub fn place_dir(place: ShortcutPlace) -> Option<PathBuf> {
        let id = match place {
            ShortcutPlace::Desktop => &FOLDERID_Desktop,
            ShortcutPlace::StartMenu => &FOLDERID_Programs,
        };
        let raw = unsafe { SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None) }.ok()?;
        let dir = unsafe { raw.to_string() }.ok().map(PathBuf::from);
        unsafe { CoTaskMemFree(Some(raw.0 as *const _)) };
        let dir = dir?;
        Some(match place {
            ShortcutPlace::Desktop => dir,
            ShortcutPlace::StartMenu => dir.join("Questkeep Games"),
        })
    }

    pub fn write(path: &Path, spec: &ShortcutSpec) -> Result<()> {
        let _com = Com::init();
        save(path, spec)
            .map_err(|e| CoreError::Invalid(format!("Could not create the shortcut: {e}")))
    }

    fn save(path: &Path, spec: &ShortcutSpec) -> windows::core::Result<()> {
        unsafe {
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
            link.SetPath(&HSTRING::from(spec.target.as_os_str()))?;
            link.SetArguments(&HSTRING::from(spec.args.as_str()))?;
            let working = spec
                .working_dir
                .as_deref()
                .or_else(|| spec.target.parent())
                .filter(|p| !p.as_os_str().is_empty());
            if let Some(dir) = working {
                link.SetWorkingDirectory(&HSTRING::from(dir.as_os_str()))?;
            }
            if let Some(text) = spec.description.as_deref().filter(|d| !d.is_empty()) {
                link.SetDescription(&HSTRING::from(text))?;
            }
            if let ShortcutIcon::Resource { path, index } = &spec.icon {
                link.SetIconLocation(&HSTRING::from(path.as_os_str()), *index)?;
            }
            let file: IPersistFile = link.cast()?;
            file.Save(&HSTRING::from(path.as_os_str()), true)
        }
    }

    pub fn points_to(path: &Path, target: &Path, args: &str) -> bool {
        let _com = Com::init();
        read(path).is_ok_and(|(t, a)| {
            t.eq_ignore_ascii_case(&target.to_string_lossy())
                && a.trim().eq_ignore_ascii_case(args.trim())
        })
    }

    fn read(path: &Path) -> windows::core::Result<(String, String)> {
        unsafe {
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
            let file: IPersistFile = link.cast()?;
            file.Load(&HSTRING::from(path.as_os_str()), STGM_READ)?;
            let mut target = [0u16; 1024];
            link.GetPath(&mut target, std::ptr::null_mut(), 0)?;
            let mut args = [0u16; 4096];
            link.GetArguments(&mut args)?;
            Ok((from_wide(&target), from_wide(&args)))
        }
    }

    fn from_wide(buf: &[u16]) -> String {
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(stem: &str) -> String {
        if cfg!(windows) {
            format!("{stem}.lnk")
        } else {
            format!("questkeep-{stem}.desktop")
        }
    }

    fn png_bytes(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(w, h, image::Rgba([1, 2, 3, 255]));
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        png
    }

    #[test]
    fn file_name_strips_and_limits() {
        assert_eq!(file_name("Half-Life 2"), named("Half-Life 2"));
        assert_eq!(file_name("Who: \"Dr\"? <A>/B\\C|D*"), named("Who Dr ABCD"));
        assert_eq!(file_name("tab\there\n"), named("tabhere"));
        assert_eq!(file_name("Game... "), named("Game"));
        assert_eq!(file_name(""), named("Game"));
        assert_eq!(file_name(" ?:. "), named("Game"));
        assert_eq!(file_name(&"x".repeat(300)), named(&"x".repeat(100)));
    }

    #[test]
    fn file_name_avoids_device_names() {
        assert_eq!(file_name("CON"), named("CON (game)"));
        assert_eq!(file_name("nul"), named("nul (game)"));
        assert_eq!(file_name("com1"), named("com1 (game)"));
        assert_eq!(file_name("Console"), named("Console"));
    }

    #[test]
    fn png_to_ico_wraps_the_png() {
        let png = png_bytes(16, 8);
        let ico = png_to_ico(&png).unwrap();
        assert_eq!(&ico[..6], &[0, 0, 1, 0, 1, 0]);
        assert_eq!((ico[6], ico[7]), (16, 8));
        assert_eq!(
            u32::from_le_bytes(ico[14..18].try_into().unwrap()) as usize,
            png.len()
        );
        assert_eq!(u32::from_le_bytes(ico[18..22].try_into().unwrap()), 22);
        assert_eq!(&ico[22..], &png[..]);

        let ico = png_to_ico(&png_bytes(256, 256)).unwrap();
        assert_eq!((ico[6], ico[7]), (0, 0));

        assert!(png_to_ico(b"not a png at all, definitely not a png").is_err());
    }

    #[test]
    fn desktop_entry_escapes_exec() {
        let spec = ShortcutSpec {
            name: "My Game".into(),
            target: PathBuf::from("/opt/Quest \"keep\"/$x`y`\\z%.exe"),
            args: "--start abc".into(),
            working_dir: None,
            description: Some("Play".into()),
            icon: ShortcutIcon::Png(PathBuf::from("/tmp/i.png")),
        };
        let text = desktop_entry(&spec);
        assert!(text.starts_with("[Desktop Entry]\nType=Application\nName=My Game\n"));
        assert!(text.contains("Comment=Play\n"));
        assert!(text.contains("Icon=/tmp/i.png\n"));
        assert!(text.contains("Terminal=false\nCategories=Game;\n"));
        assert!(text.contains(
            "Exec=\"/opt/Quest \\\\\"keep\\\\\"/\\\\$x\\\\`y\\\\`\\\\\\\\z%%.exe\" --start abc\n"
        ));
        assert!(desktop_points_to(&text, &spec.target, "--start abc"));
        assert!(!desktop_points_to(&text, &spec.target, "--start other"));
    }

    #[cfg(unix)]
    #[test]
    fn user_dirs_file_is_parsed() {
        let home = Path::new("/home/u");
        let text = "# c\nXDG_DOWNLOAD_DIR=\"$HOME/Dl\"\nXDG_DESKTOP_DIR=\"$HOME/Schreibtisch\"\n";
        assert_eq!(
            parse_desktop_dir(text, home),
            Some(PathBuf::from("/home/u/Schreibtisch"))
        );
        assert_eq!(
            parse_desktop_dir("XDG_DESKTOP_DIR=\"/x/d\"", home),
            Some(PathBuf::from("/x/d"))
        );
        assert_eq!(parse_desktop_dir("nothing", home), None);
    }

    #[cfg(windows)]
    #[test]
    fn windows_shortcut_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let target = std::env::current_exe().unwrap();
        let spec = ShortcutSpec {
            name: "Test Game".into(),
            target: target.clone(),
            args: "--start 123".into(),
            working_dir: None,
            description: Some("desc".into()),
            icon: ShortcutIcon::Resource {
                path: target.clone(),
                index: 0,
            },
        };
        let first = create(dir.path(), &spec, None).unwrap();
        assert_eq!(first, dir.path().join("Test Game.lnk"));
        assert!(points_to(&first, &target, "--start 123"));
        assert!(!points_to(&first, &target, "--start 456"));
        assert!(!points_to(
            &first,
            Path::new("C:\\other.exe"),
            "--start 123"
        ));
        assert!(!points_to(
            &dir.path().join("missing.lnk"),
            &target,
            "--start 123"
        ));

        let second = create(dir.path(), &spec, None).unwrap();
        assert_eq!(second, dir.path().join("Test Game (2).lnk"));
        let third = create(dir.path(), &spec, None).unwrap();
        assert_eq!(third, dir.path().join("Test Game (3).lnk"));

        let mut changed = spec.clone();
        changed.args = "--start 456".into();
        let again = create(dir.path(), &changed, Some(&first)).unwrap();
        assert_eq!(again, first);
        assert!(points_to(&first, &target, "--start 456"));

        assert!(place_dir(ShortcutPlace::Desktop).unwrap().is_absolute());
        assert!(
            place_dir(ShortcutPlace::StartMenu)
                .unwrap()
                .ends_with("Questkeep Games")
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn linux_shortcut_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let spec = ShortcutSpec {
            name: "Test".into(),
            target: PathBuf::from("/usr/bin/questkeep"),
            args: "--start 1".into(),
            working_dir: None,
            description: None,
            icon: ShortcutIcon::None,
        };
        let path = create(dir.path(), &spec, None).unwrap();
        assert!(points_to(&path, &spec.target, "--start 1"));
        assert!(!points_to(&path, &spec.target, "--start 2"));
        let second = create(dir.path(), &spec, None).unwrap();
        assert!(
            second
                .to_string_lossy()
                .ends_with("questkeep-Test (2).desktop")
        );
    }
}
