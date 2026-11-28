//! Requests from outside the window: `questkeep://` links and `--start`/`--show`/`--search` on
//! the command line (parsed in `questkeep_core::external`), and the tray's game actions. Starting
//! a game runs off the UI thread; a game page or a search is kept until the window takes it (a
//! window that waited in the tray is built first and asks once it has loaded).

use std::sync::{Arc, Mutex, PoisonError};

use questkeep_core::events::CoreEvent;
use questkeep_core::external::ExternalRequest;
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

use crate::AppState;
use crate::window;

static GAME_PAGE: Mutex<Option<Uuid>> = Mutex::new(None);
static SEARCH: Mutex<Option<String>> = Mutex::new(None);
/// Tells a loaded window that [`GAME_PAGE`] has a game for it.
const SHOW_GAME_EVENT: &str = "show-game";
/// Tells a loaded window that [`SEARCH`] has a search for it.
const SEARCH_EVENT: &str = "search-games";

/// The game page asked for since the last call, if any.
pub fn take_game_page_request() -> Option<Uuid> {
    GAME_PAGE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take()
}

/// The search asked for since the last call, if any.
pub fn take_search_request() -> Option<String> {
    SEARCH.lock().unwrap_or_else(PoisonError::into_inner).take()
}

/// Brings the window to the front on the game's page.
pub fn open_game_page(app: &AppHandle, id: Uuid) {
    *GAME_PAGE.lock().unwrap_or_else(PoisonError::into_inner) = Some(id);
    window::show_main_window(app);
    // A window that is already loaded hears the event; a new one asks when it has loaded.
    let _ = app.emit(SHOW_GAME_EVENT, ());
}

/// Brings the window to the front with Find a game open on `text`.
fn open_search(app: &AppHandle, text: String) {
    *SEARCH.lock().unwrap_or_else(PoisonError::into_inner) = Some(text);
    window::show_main_window(app);
    let _ = app.emit(SEARCH_EVENT, ());
}

/// Launches a game like the `launch_game` command, with a launch profile or its default; a
/// failure is logged and, if the window is open, shown there as an error toast.
pub fn launch(app: &AppHandle, id: Uuid, profile: Option<Uuid>) {
    let Some(state) = app.try_state::<Arc<AppState>>() else {
        return;
    };
    let state = Arc::clone(&state);
    std::thread::spawn(move || {
        let result = state
            .library
            .launch_target(id, profile)
            .and_then(|game| state.launcher.launch(&game));
        if let Err(err) = result {
            tracing::warn!(%err, %id, "launching from outside the window failed");
            state.library.events().publish(CoreEvent::GameError {
                id,
                message: err.to_string(),
            });
        }
    });
}

/// Carries out a link or command-line request. A start link only starts an installed game, with
/// its own play action: anything else (a game that is not installed, an unknown ID) opens its
/// page instead, so a link from a web page can never install, uninstall or run something else.
pub fn handle(app: &AppHandle, request: ExternalRequest) {
    tracing::info!(?request, "request from outside");
    let app = app.clone();
    // Off the calling thread: showing the window waits for it, and the setup hook is on the UI
    // thread.
    std::thread::spawn(move || match request {
        ExternalRequest::Start { game, profile } => {
            let installed = app
                .try_state::<Arc<AppState>>()
                .and_then(|state| state.library.launch_target(game, profile).ok())
                .is_some_and(|target| target.installed);
            if installed {
                launch(&app, game, profile);
            } else {
                open_game_page(&app, game);
            }
        }
        ExternalRequest::Show { game } => open_game_page(&app, game),
        ExternalRequest::Search { text } => open_search(&app, text),
    });
}

/// Whether a start of the app for `request` may stay in the tray instead of opening the window
/// (only a start link: the game's window takes the focus anyway).
pub fn stays_in_tray(request: Option<&ExternalRequest>) -> bool {
    matches!(request, Some(ExternalRequest::Start { .. }))
}
