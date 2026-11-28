//! Where a game's manual is: a web address, or a file (absolute, or relative to the install
//! folder).

use std::path::{Path, PathBuf};

use crate::error::{CoreError, Result};

/// What "Open manual" opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManualTarget {
    /// An http(s) address, opened in the browser.
    Url(String),
    /// An existing file, opened with its system handler.
    File(PathBuf),
}

/// Whether `text` is an http or https address.
pub fn is_web_address(text: &str) -> bool {
    let lower = text.trim().to_ascii_lowercase();
    ["http://", "https://"].iter().any(|scheme| {
        lower
            .strip_prefix(scheme)
            .is_some_and(|rest| !rest.is_empty() && !rest.contains(char::is_whitespace))
    })
}

/// Resolves the manual of a game: a web address as it is, a path against `install_dir` when it is
/// relative. Fails with a readable message when there is no manual or the file is missing.
pub fn resolve(manual: Option<&str>, install_dir: Option<&str>) -> Result<ManualTarget> {
    let manual = manual.map(str::trim).filter(|m| !m.is_empty());
    let Some(manual) = manual else {
        return Err(CoreError::Invalid("This game has no manual set.".into()));
    };
    if is_web_address(manual) {
        return Ok(ManualTarget::Url(manual.to_string()));
    }
    let path = Path::new(manual);
    let full = if path.is_absolute() {
        path.to_path_buf()
    } else {
        match install_dir.filter(|d| !d.trim().is_empty()) {
            Some(dir) => Path::new(dir).join(path),
            None => {
                return Err(CoreError::Invalid(format!(
                    "The manual \"{manual}\" is relative to the game's install folder, which is unknown."
                )));
            }
        }
    };
    if full.is_file() {
        Ok(ManualTarget::File(full))
    } else {
        Err(CoreError::Invalid(format!(
            "The manual was not found: {}",
            full.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_addresses_open_as_they_are() {
        assert_eq!(
            resolve(Some(" https://example.org/manual.pdf "), None).unwrap(),
            ManualTarget::Url("https://example.org/manual.pdf".into())
        );
        assert!(is_web_address("HTTP://example.org"));
        assert!(!is_web_address("ftp://example.org"));
        assert!(!is_web_address("https://"));
    }

    #[test]
    fn relative_paths_resolve_against_the_install_folder() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("docs")).unwrap();
        std::fs::write(tmp.path().join("docs").join("manual.pdf"), b"x").unwrap();
        let dir = tmp.path().to_string_lossy().to_string();
        assert_eq!(
            resolve(Some("docs/manual.pdf"), Some(&dir)).unwrap(),
            ManualTarget::File(tmp.path().join("docs/manual.pdf"))
        );
        let absolute = tmp.path().join("docs").join("manual.pdf");
        assert_eq!(
            resolve(Some(&absolute.to_string_lossy()), None).unwrap(),
            ManualTarget::File(absolute)
        );
    }

    #[test]
    fn missing_manuals_are_readable_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_string_lossy().to_string();
        assert!(resolve(None, None).is_err());
        assert!(resolve(Some("  "), None).is_err());
        assert!(resolve(Some("manual.pdf"), None).is_err());
        let err = resolve(Some("manual.pdf"), Some(&dir)).unwrap_err();
        assert!(err.to_string().contains("not found"), "{err}");
    }
}
