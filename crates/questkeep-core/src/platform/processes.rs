//! Cheap process listing for noticing games started outside Questkeep. Listing returns only ids
//! (`EnumProcesses` on Windows: microseconds, where a ToolHelp snapshot of every process and thread
//! takes milliseconds); a process is opened and its paths read once, when it first shows up.
//!
//! On Windows the opened handle is kept: an id can't be reused while a handle to its process is
//! open, so a known id always means the same process, and an exited process is noticed through
//! its handle (exited processes still referenced by handles may stay listed).
//!
//! One of the few modules with `unsafe` code (see docs/DECISIONS.md): plain Win32 calls with local
//! buffers and handles owned by [`ProcessRef`].

#![cfg_attr(windows, allow(unsafe_code))]

/// A process opened by [`open_process`]; closes its handle when dropped.
#[derive(Debug)]
pub struct ProcessRef {
    /// The process handle as an integer (raw handles are not `Send`); 0 when not opened.
    #[cfg(windows)]
    handle: isize,
}

impl ProcessRef {
    /// A reference that holds nothing (processes that can't be opened, tests).
    pub fn none() -> Self {
        Self {
            #[cfg(windows)]
            handle: 0,
        }
    }

    /// Whether the process is known to have exited.
    pub fn has_exited(&self) -> bool {
        #[cfg(windows)]
        {
            use windows_sys::Win32::Foundation::WAIT_OBJECT_0;
            use windows_sys::Win32::System::Threading::WaitForSingleObject;
            if self.handle == 0 {
                return false;
            }
            // SAFETY: the handle is owned by `self` and open until drop.
            unsafe { WaitForSingleObject(self.handle as _, 0) == WAIT_OBJECT_0 }
        }
        #[cfg(not(windows))]
        {
            false
        }
    }
}

#[cfg(windows)]
impl Drop for ProcessRef {
    fn drop(&mut self) {
        if self.handle != 0 {
            // SAFETY: the handle is owned by `self` and closed exactly once.
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.handle as _);
            }
        }
    }
}

/// Replaces `out` with the ids of the running processes.
#[cfg(windows)]
pub fn list_processes(out: &mut Vec<u32>) {
    use windows_sys::Win32::System::ProcessStatus::EnumProcesses;
    if out.capacity() < 1024 {
        out.reserve(1024 - out.len());
    }
    loop {
        out.resize(out.capacity(), 0);
        let size = u32::try_from(out.len() * size_of::<u32>()).unwrap_or(u32::MAX);
        let mut written = 0u32;
        // SAFETY: the buffer is a local vector whose byte size is passed along.
        let ok = unsafe { EnumProcesses(out.as_mut_ptr(), size, &mut written) } != 0;
        if !ok {
            out.clear();
            return;
        }
        let count = written as usize / size_of::<u32>();
        // A full buffer may have cut the list short: grow and ask again.
        if count < out.len() {
            out.truncate(count);
            return;
        }
        out.reserve(out.len());
    }
}

/// Opens a process and returns the paths identifying it (its executable, then its working
/// directory when it can be read) and the reference that keeps its id. No paths when the process
/// is gone or protected.
#[cfg(windows)]
pub fn open_process(pid: u32) -> (Vec<String>, ProcessRef) {
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
        QueryFullProcessImageNameW,
    };
    // SAFETY: the buffer is local and its length is passed along; the handle is owned by the
    // returned `ProcessRef`, which closes it.
    unsafe {
        let handle = OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
            0,
            pid,
        );
        if handle.is_null() {
            return (Vec::new(), ProcessRef::none());
        }
        let process = ProcessRef {
            handle: handle as isize,
        };
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        if QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len) == 0 {
            return (Vec::new(), process);
        }
        let len = (len as usize).min(buf.len());
        let mut paths = vec![String::from_utf16_lossy(&buf[..len])];
        paths.extend(working_dir(pid));
        (paths, process)
    }
}

/// The working directory of a process, read from its process parameters (Windows keeps no other
/// record of it). Games run by a shared program (Minecraft in `javaw.exe` from a Java folder) are
/// told apart by it, like `/proc/<pid>/cwd` does on Linux. `None` when the process can't be read
/// (protected, elevated while Questkeep isn't, gone).
#[cfg(windows)]
fn working_dir(pid: u32) -> Option<String> {
    use windows_sys::Wdk::System::Threading::{
        NtQueryInformationProcess, ProcessBasicInformation, ProcessWow64Information,
    };
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
    };

    /// Closes the handle when dropped.
    struct Handle(windows_sys::Win32::Foundation::HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            // SAFETY: the handle was opened below and is closed exactly once.
            unsafe { CloseHandle(self.0) };
        }
    }

    /// Reads `len` bytes at `address` in the process.
    fn read(process: &Handle, address: u64, len: usize) -> Option<Vec<u8>> {
        let address = usize::try_from(address).ok().filter(|a| *a != 0)?;
        let mut buf = vec![0u8; len];
        let mut read = 0usize;
        // SAFETY: the buffer is a local vector whose length is passed along; reading another
        // process's memory can fail but not corrupt ours.
        let ok = unsafe {
            ReadProcessMemory(
                process.0,
                address as *const _,
                buf.as_mut_ptr().cast(),
                len,
                &mut read,
            )
        } != 0;
        (ok && read == len).then_some(buf)
    }
    fn u32_at(bytes: &[u8], at: usize) -> u64 {
        bytes
            .get(at..at + 4)
            .and_then(|b| b.try_into().ok())
            .map_or(0, |b| u64::from(u32::from_le_bytes(b)))
    }
    fn u64_at(bytes: &[u8], at: usize) -> u64 {
        bytes
            .get(at..at + 8)
            .and_then(|b| b.try_into().ok())
            .map_or(0, u64::from_le_bytes)
    }

    // SAFETY: the handle is owned by `Handle`, which closes it.
    let handle =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ, 0, pid) };
    if handle.is_null() {
        return None;
    }
    let process = Handle(handle);

    // A 32-bit process on 64-bit Windows keeps its own 32-bit PEB; the native one's parameters
    // are not updated when it changes its directory.
    let mut peb32 = 0usize;
    // SAFETY: the out buffer is a local `usize`, its size passed along.
    let status = unsafe {
        NtQueryInformationProcess(
            process.0,
            ProcessWow64Information,
            (&raw mut peb32).cast(),
            size_of::<usize>() as u32,
            std::ptr::null_mut(),
        )
    };
    // Offsets of PEB.ProcessParameters, RTL_USER_PROCESS_PARAMETERS.CurrentDirectory.DosPath
    // (a UNICODE_STRING: byte length, then the buffer address at the given offset).
    let (params, dos_path, buffer_at) = if status >= 0 && peb32 != 0 {
        let peb = read(&process, peb32 as u64 + 0x10, 4)?;
        (u32_at(&peb, 0), 0x24, 4)
    } else if cfg!(target_pointer_width = "64") {
        // PROCESS_BASIC_INFORMATION: the PEB address is its second field.
        let mut info = [0usize; 6];
        // SAFETY: the out buffer is a local array, its size passed along.
        let status = unsafe {
            NtQueryInformationProcess(
                process.0,
                ProcessBasicInformation,
                info.as_mut_ptr().cast(),
                size_of_val(&info) as u32,
                std::ptr::null_mut(),
            )
        };
        if status < 0 {
            return None;
        }
        let peb = read(&process, info[1] as u64 + 0x20, 8)?;
        (u64_at(&peb, 0), 0x38, 8)
    } else {
        return None;
    };
    let string = read(&process, params + dos_path, 16)?;
    let len = usize::from(u16::from_le_bytes([string[0], string[1]]));
    let buffer = if buffer_at == 8 {
        u64_at(&string, 8)
    } else {
        u32_at(&string, 4)
    };
    if len == 0 || len % 2 != 0 {
        return None;
    }
    let bytes = read(&process, buffer, len)?;
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    let dir = String::from_utf16_lossy(&units);
    (!dir.is_empty()).then_some(dir)
}

/// Replaces `out` with the ids of the running processes.
#[cfg(target_os = "linux")]
pub fn list_processes(out: &mut Vec<u32>) {
    out.clear();
    let Ok(dir) = std::fs::read_dir("/proc") else {
        return;
    };
    for entry in dir.flatten() {
        if let Some(pid) = entry.file_name().to_str().and_then(|n| n.parse().ok()) {
            out.push(pid);
        }
    }
}

/// Paths identifying a process: its executable, working directory and path-like arguments
/// (Proton games run as `wine64-preloader` and only those point into the game folder). Linux
/// hands out ids in increasing order, so no reference is needed to keep one.
#[cfg(target_os = "linux")]
pub fn open_process(pid: u32) -> (Vec<String>, ProcessRef) {
    let base = format!("/proc/{pid}");
    let link = |name: &str| {
        std::fs::read_link(format!("{base}/{name}"))
            .ok()
            .map(|p| p.display().to_string())
    };
    let mut paths: Vec<String> = link("exe").into_iter().collect();
    let cwd = link("cwd");
    let args = std::fs::read(format!("{base}/cmdline")).unwrap_or_default();
    let args = args
        .split(|&b| b == 0)
        .filter(|a| !a.is_empty())
        .map(|a| String::from_utf8_lossy(a).into_owned());
    paths.extend(crate::launcher::path_hints(cwd, args));
    (paths, ProcessRef::none())
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn list_processes(out: &mut Vec<u32>) {
    out.clear();
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn open_process(_pid: u32) -> (Vec<String>, ProcessRef) {
    (Vec::new(), ProcessRef::none())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn lists_and_opens_the_current_process() {
        let me = std::process::id();
        let mut pids = Vec::new();
        list_processes(&mut pids);
        assert!(pids.contains(&me));
        let exe = std::env::current_exe().unwrap();
        let (paths, process) = open_process(me);
        let name = exe.file_name().unwrap().to_string_lossy().to_string();
        assert!(paths.iter().any(|p| p.ends_with(&name)), "{paths:?}");
        assert!(!process.has_exited());
    }

    #[cfg(windows)]
    #[test]
    fn notices_an_exited_process_through_its_handle() {
        let mut child = std::process::Command::new("cmd.exe")
            .args(["/C", "exit"])
            .spawn()
            .unwrap();
        let (_, process) = open_process(child.id());
        child.wait().unwrap();
        assert!(process.has_exited());
    }

    /// A 64-bit and a 32-bit (WOW64) process report the folder they were started in.
    #[cfg(windows)]
    #[test]
    fn reads_the_working_directory_of_native_and_32_bit_processes() {
        use std::os::windows::process::CommandExt;
        let dir = tempfile::tempdir().unwrap();
        let system = std::env::var("SystemRoot").unwrap();
        let mut shells = vec![format!("{system}\\System32\\cmd.exe")];
        let wow64 = format!("{system}\\SysWOW64\\cmd.exe");
        if std::path::Path::new(&wow64).exists() {
            shells.push(wow64);
        }
        for shell in shells {
            // Waits for input that never comes; no window (CREATE_NO_WINDOW).
            let mut child = std::process::Command::new(&shell)
                .args(["/Q", "/K"])
                .current_dir(dir.path())
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .creation_flags(0x0800_0000)
                .spawn()
                .unwrap();
            let (paths, _) = open_process(child.id());
            child.kill().unwrap();
            child.wait().unwrap();
            let expected = crate::launcher::normalize_dir(&dir.path().display().to_string());
            assert!(
                paths
                    .iter()
                    .skip(1)
                    .any(|p| crate::launcher::normalize_dir(p) == expected),
                "{shell}: {paths:?}"
            );
        }
    }
}
