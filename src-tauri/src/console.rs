//! Debug builds are console programs (release builds use the Windows subsystem, see `main.rs`).
//! Started from a terminal (`pnpm dev`) they share its console and log there; started directly
//! (a desktop shortcut, a `questkeep://` link, Explorer) Windows opens a console just for them,
//! which would stay open showing the logs. That one is let go at start, which closes it.

/// Lets go of a console no other process uses. Debug builds on Windows only.
#[cfg(all(windows, debug_assertions))]
#[allow(unsafe_code)]
pub fn release_own_console() {
    use windows_sys::Win32::System::Console::{FreeConsole, GetConsoleProcessList};
    let mut processes = [0u32; 2];
    // SAFETY: the buffer is valid for the length passed; both calls take no other pointers.
    let attached = unsafe { GetConsoleProcessList(processes.as_mut_ptr(), processes.len() as u32) };
    // 1: only this process (a console made for it); 0: none or an error; more: a terminal's.
    if attached == 1 {
        // SAFETY: no Rust handles to the console are kept; logging falls back to the log file.
        unsafe { FreeConsole() };
    }
}

#[cfg(not(all(windows, debug_assertions)))]
pub fn release_own_console() {}
