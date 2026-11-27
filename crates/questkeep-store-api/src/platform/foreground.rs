//! Foreground hand-off and key presses for launcher dialogs the user asked for (an add-game option
//! whose launcher has no command-line option for it, e.g. Prism Launcher's Ctrl+N "Add
//! Instance"). Keys are only sent while a window of the expected program is in front, checked
//! before every combination, so nothing reaches another program.
//!
//! One of the few modules with `unsafe` code (see docs/DECISIONS.md): plain Win32 calls with local
//! buffers.

#![allow(unsafe_code)]

use crate::model::{Key, KeyCombo};

/// Lets the process started next take the foreground (the user just asked for it).
pub(super) fn allow_foreground_handoff() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{ASFW_ANY, AllowSetForegroundWindow};
    // SAFETY: plain Win32 call without pointers.
    unsafe {
        AllowSetForegroundWindow(ASFW_ANY);
    }
}

/// The foreground window's title and the file name of its process.
fn foreground_window() -> Option<(String, String)> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
    };
    // SAFETY: every buffer is local and its length is passed along; the process handle is
    // closed before returning.
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut title = [0u16; 512];
        let len = GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32);
        let title = String::from_utf16_lossy(&title[..usize::try_from(len).ok()?]);
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }
        let mut path = [0u16; 1024];
        let mut size = path.len() as u32;
        let ok =
            QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, path.as_mut_ptr(), &mut size);
        CloseHandle(process);
        if ok == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&path[..size as usize]);
        let exe = path.rsplit('\\').next().unwrap_or_default().to_string();
        Some((title, exe))
    }
}

/// The Windows virtual-key code of `key` and whether it is an extended key.
fn virtual_key(key: Key) -> (u16, bool) {
    match key {
        Key::Letter(c) | Key::Digit(c) => (c as u16, false),
        Key::Function(n) => (0x6F + u16::from(n), false), // VK_F1 = 0x70
        Key::Enter => (0x0D, false),
        Key::Tab => (0x09, false),
        Key::Escape => (0x1B, false),
        Key::Space => (0x20, false),
        Key::Backspace => (0x08, false),
        Key::PageUp => (0x21, true),
        Key::PageDown => (0x22, true),
        Key::End => (0x23, true),
        Key::Home => (0x24, true),
        Key::Left => (0x25, true),
        Key::Up => (0x26, true),
        Key::Right => (0x27, true),
        Key::Down => (0x28, true),
        Key::Insert => (0x2D, true),
        Key::Delete => (0x2E, true),
    }
}

/// Presses one combination: modifiers down, the key down and up, modifiers up, in one
/// `SendInput` call so nothing can come in between. True when all of it was sent.
fn send_combo(combo: &KeyCombo) -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
        KEYEVENTF_KEYUP, SendInput, VK_CONTROL, VK_LWIN, VK_MENU, VK_SHIFT,
    };
    let input = |vk: u16, flags: KEYBD_EVENT_FLAGS| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let modifiers: Vec<(u16, KEYBD_EVENT_FLAGS)> = [
        (combo.ctrl, VK_CONTROL, 0),
        (combo.shift, VK_SHIFT, 0),
        (combo.alt, VK_MENU, 0),
        (combo.meta, VK_LWIN, KEYEVENTF_EXTENDEDKEY),
    ]
    .into_iter()
    .filter(|(on, _, _)| *on)
    .map(|(_, vk, flags)| (vk, flags))
    .collect();
    let (vk, extended) = virtual_key(combo.key);
    let key_flags = if extended { KEYEVENTF_EXTENDEDKEY } else { 0 };
    let mut inputs: Vec<INPUT> = modifiers.iter().map(|&(m, f)| input(m, f)).collect();
    inputs.push(input(vk, key_flags));
    inputs.push(input(vk, key_flags | KEYEVENTF_KEYUP));
    inputs.extend(
        modifiers
            .iter()
            .rev()
            .map(|&(m, f)| input(m, f | KEYEVENTF_KEYUP)),
    );
    // SAFETY: `inputs` is a valid slice and its exact length and element size are passed.
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    sent == inputs.len() as u32
}

/// Waits up to `timeout` for a window of `exe` whose title starts with `title_prefix` to be in
/// front, then presses `steps` (wait in milliseconds, combination) in order. Before every
/// combination a window of `exe` must be in front (the first one also needs the title; later ones
/// may be a dialog the earlier keys opened); otherwise it stops. Never sends anything to another
/// program.
pub(super) fn press_keys_in(
    exe: &str,
    title_prefix: &str,
    steps: &[(u32, KeyCombo)],
    timeout: std::time::Duration,
) -> Result<(), String> {
    use std::time::{Duration, Instant};
    let in_front = |need_title: bool| {
        foreground_window().is_some_and(|(title, name)| {
            name.eq_ignore_ascii_case(exe) && (!need_title || title.starts_with(title_prefix))
        })
    };
    let deadline = Instant::now() + timeout;
    loop {
        if in_front(true) {
            // Let the window finish appearing, then check again before typing.
            std::thread::sleep(Duration::from_millis(400));
            if in_front(true) {
                break;
            }
        }
        if Instant::now() >= deadline {
            return Err("the launcher's window did not come to the front".into());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    for (index, (delay_ms, combo)) in steps.iter().enumerate() {
        std::thread::sleep(Duration::from_millis(u64::from(*delay_ms)));
        if !in_front(index == 0) {
            return Err(format!(
                "another window was in front before key step {}; stopped",
                index + 1
            ));
        }
        if !send_combo(combo) {
            return Err(format!("Windows did not take key step {}", index + 1));
        }
    }
    Ok(())
}
