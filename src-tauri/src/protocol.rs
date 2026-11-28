//! Registers this executable as the handler of `questkeep://` links for the current user, at
//! every start of a release build, so the installed app and the portable exe (wherever it was
//! moved) both work; the one started last handles links. Windows: `HKCU\Software\Classes\
//! questkeep` (the NSIS uninstaller removes it, see `windows/hooks.nsh`). Linux: a `.desktop`
//! file for `x-scheme-handler/questkeep`, made the default with `xdg-mime`.
//!
//! Thin glue that writes to the user's real registry or desktop database, so it is not
//! unit-tested; debug builds only register with `QUESTKEEP_REGISTER_PROTOCOL=1` (a dev build must
//! not take over the installed app's links).

use questkeep_core::external::URI_SCHEME;

/// Registers the link handler in a background thread; failures are logged.
pub fn register_on_startup() {
    let wanted = !cfg!(debug_assertions)
        || std::env::var("QUESTKEEP_REGISTER_PROTOCOL").is_ok_and(|v| v == "1");
    if !wanted {
        return;
    }
    std::thread::spawn(|| match std::env::current_exe() {
        Ok(exe) => {
            if let Err(err) = register(&exe) {
                tracing::warn!(%err, "could not register the questkeep:// link handler");
            }
        }
        Err(err) => tracing::warn!(%err, "no executable path for the link handler"),
    });
}

#[cfg(windows)]
fn register(exe: &std::path::Path) -> std::io::Result<()> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let exe = exe.display();
    let classes = RegKey::predef(HKEY_CURRENT_USER).open_subkey(r"Software\Classes")?;
    let (key, _) = classes.create_subkey(URI_SCHEME)?;
    key.set_value("", &"URL:Questkeep")?;
    key.set_value("URL Protocol", &"")?;
    let (icon, _) = key.create_subkey("DefaultIcon")?;
    icon.set_value("", &format!("\"{exe}\",0"))?;
    let (command, _) = key.create_subkey(r"shell\open\command")?;
    command.set_value("", &format!("\"{exe}\" \"%1\""))?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn register(exe: &std::path::Path) -> std::io::Result<()> {
    const FILE: &str = "questkeep-url-handler.desktop";
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::Path::new(&home).join(".local/share"))
        })
        .ok_or_else(|| std::io::Error::other("no XDG_DATA_HOME or HOME"))?;
    let dir = data.join("applications");
    std::fs::create_dir_all(&dir)?;
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=Questkeep\nNoDisplay=true\n\
         Exec=\"{}\" %u\nMimeType=x-scheme-handler/{URI_SCHEME};\n",
        exe.display()
    );
    std::fs::write(dir.join(FILE), entry)?;
    // Missing xdg-utils only means links do not open Questkeep; nothing else depends on it.
    let status = std::process::Command::new("xdg-mime")
        .args(["default", FILE, &format!("x-scheme-handler/{URI_SCHEME}")])
        .status()?;
    if !status.success() {
        tracing::warn!(
            ?status,
            "xdg-mime could not make Questkeep the link handler"
        );
    }
    Ok(())
}

#[cfg(not(any(windows, target_os = "linux")))]
fn register(_exe: &std::path::Path) -> std::io::Result<()> {
    Ok(())
}
