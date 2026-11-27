//! OS helpers stores need: the registry, the Windows "Apps" list, Store app packages.
//! Everything is behind `cfg` so stores stay portable.

use std::path::PathBuf;

#[cfg(windows)]
mod drives;
#[cfg(windows)]
mod foreground;
#[cfg(windows)]
mod windows;

/// One program of the Windows "Apps" list (an `...\CurrentVersion\Uninstall\<key>` entry).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UninstallEntry {
    /// The registry key name (a product code, a GUID, or the program's own id).
    pub key: String,
    pub display_name: Option<String>,
    pub publisher: Option<String>,
    pub install_location: Option<String>,
    pub uninstall_string: Option<String>,
    pub display_icon: Option<String>,
}

/// The Windows "Apps" list (machine and user entries); empty elsewhere.
pub fn uninstall_entries() -> Vec<UninstallEntry> {
    #[cfg(windows)]
    {
        windows::uninstall_entries()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// A registry string given as `HKEY_LOCAL_MACHINE\Key\Path\Value`; `None` elsewhere or when
/// missing.
pub fn registry_value(path: &str) -> Option<String> {
    #[cfg(windows)]
    {
        windows::registry_value(path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        None
    }
}

/// The names of the subkeys of a registry key given as `HKEY_LOCAL_MACHINE\Some\Key`; empty
/// elsewhere or when the key is missing.
pub fn registry_subkeys(path: &str) -> Vec<String> {
    #[cfg(windows)]
    {
        windows::registry_subkeys(path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Vec::new()
    }
}

/// The roots of the local drives (`C:\`): fixed disks and removable ones. Windows only; empty
/// elsewhere (the file system has one root).
pub fn local_drives() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        drives::fixed_drives()
    }
    #[cfg(not(windows))]
    Vec::new()
}

/// A command line such as an `UninstallString` (`"C:\x\y.exe" /a b` or `C:\x\y.exe /a`) as a
/// program and its argument string.
pub fn split_command_line(command: &str) -> Option<(String, String)> {
    let command = command.trim();
    let (program, rest) = if let Some(quoted) = command.strip_prefix('"') {
        let (program, rest) = quoted.split_once('"')?;
        (program.to_string(), rest)
    } else {
        // Unquoted paths may contain spaces; the program ends after ".exe".
        let lower = command.to_ascii_lowercase();
        match lower.find(".exe") {
            Some(end) => (command[..end + 4].to_string(), &command[end + 4..]),
            None => match command.split_once(' ') {
                Some((program, rest)) => (program.to_string(), rest),
                None => (command.to_string(), ""),
            },
        }
    };
    (!program.is_empty()).then(|| (program, rest.trim().to_string()))
}

/// Lets the program started next come to the front: the user just asked for it (a launcher
/// opened from a menu). Windows only; elsewhere nothing to do.
pub fn allow_foreground_handoff() {
    #[cfg(windows)]
    foreground::allow_foreground_handoff();
}

/// Presses key combinations in the program `exe` once one of its windows whose title starts with
/// `title_prefix` is in front, waiting up to `timeout` for it; `steps` are (wait in
/// milliseconds, combination), see [`crate::model::LauncherKeys`]. Before every combination a
/// window of `exe` must be in front, otherwise it stops: it never types into another program.
/// The error says why not everything was pressed. Windows only.
pub fn press_keys_in(
    exe: &str,
    title_prefix: &str,
    steps: &[(u32, crate::model::KeyCombo)],
    timeout: std::time::Duration,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        foreground::press_keys_in(exe, title_prefix, steps, timeout)
    }
    #[cfg(not(windows))]
    {
        let _ = (exe, title_prefix, steps, timeout);
        Err("key presses are only supported on Windows".into())
    }
}

/// An installed AppX/MSIX package registered for the current user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppxPackage {
    /// `<Name>_<Version>_<Arch>_<ResourceId>_<PublisherHash>`.
    pub full_name: String,
    /// Folder the package is installed in (may not exist or may be unreadable).
    pub root: PathBuf,
    /// Application ids (`Application Id` in the manifest) registered for the package.
    pub app_ids: Vec<String>,
}

/// Packages registered for the current user (Windows only; empty elsewhere).
pub fn appx_packages() -> Vec<AppxPackage> {
    #[cfg(windows)]
    {
        windows::appx_packages()
    }
    #[cfg(not(windows))]
    Vec::new()
}
