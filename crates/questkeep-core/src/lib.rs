//! Questkeep core: library model, persistence, providers, launching and images.
//!
//! This crate has no dependency on Tauri or any UI so it can be unit-tested headless.

/// Store account sign-in, from the store plugin interface.
pub use questkeep_store_api::auth;
pub use questkeep_store_api::traffic;
pub mod activity;
pub mod auto_categories;
pub mod automation;
pub mod backup;
pub mod db;
pub mod error;
pub mod events;
pub mod game_state;
pub mod images;
pub mod install_folders;
pub mod installs;
pub mod launcher;
pub mod library;
pub mod live;
pub mod metadata;
pub mod model;
pub mod notifications;
pub mod paths;
pub mod platform;
pub mod price_sites;
pub mod programs;
pub mod providers;
pub mod resume;
pub mod scripts;
pub mod seed;
pub mod settings;
pub mod statistics;
pub mod tray;
pub mod user_themes;
pub mod window_placement;

pub use error::{CoreError, Result};
pub use library::{AppInfo, Library};

/// Crate version, exposed to the UI.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Environment variable the dev launchers set (`scripts/dev.mjs`, `scripts/dev-hidden.ps1`) to the
/// version the pending changesets will release, so a dev build is judged by plugins (`min_app`) as
/// the next release without touching any version file. Debug builds only.
pub const DEV_VERSION_VAR: &str = "QUESTKEEP_APP_VERSION";

/// The version this app presents: [`VERSION`], except that a debug build uses a valid `X.Y.Z` in
/// [`DEV_VERSION_VAR`]. Release builds ignore the variable.
pub fn app_version() -> String {
    if cfg!(debug_assertions)
        && let Some(version) = dev_version(std::env::var(DEV_VERSION_VAR).ok().as_deref())
    {
        return version;
    }
    VERSION.to_string()
}

/// `value` trimmed when it is a plain `X.Y.Z`.
fn dev_version(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    let parts: Vec<&str> = value.split('.').collect();
    (parts.len() == 3 && parts.iter().all(|p| p.parse::<u32>().is_ok())).then(|| value.to_string())
}

#[cfg(test)]
mod version_tests {
    use super::*;

    #[test]
    fn dev_version_accepts_only_a_plain_version() {
        assert_eq!(dev_version(Some(" 0.8.0 ")).as_deref(), Some("0.8.0"));
        assert_eq!(dev_version(Some("0.8.0-dev.1")), None);
        assert_eq!(dev_version(Some("next")), None);
        assert_eq!(dev_version(Some("")), None);
        assert_eq!(dev_version(None), None);
    }
}
