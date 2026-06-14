// ── Windows platform implementation ──────────────────────────────────
use std::fs;
use std::path::PathBuf;

pub type WindowId = isize;

// ── Win32 FFI ────────────────────────────────────────────────────────
extern "system" {
    fn GetForegroundWindow() -> isize;
    fn SetForegroundWindow(hWnd: isize) -> i32;
    fn IsWindow(hWnd: isize) -> i32;
    fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> isize;
    fn CloseHandle(hObject: isize) -> i32;
    fn WaitForSingleObject(hHandle: isize, dwMilliseconds: u32) -> u32;
    fn CreateToolhelp32Snapshot(dwFlags: u32, th32ProcessID: u32) -> isize;
    fn Process32FirstW(hSnapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
    fn Process32NextW(hSnapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
}

const PROCESS_SYNCHRONIZE: u32 = 0x00100000;
const WAIT_TIMEOUT: u32 = 0x00000102;
const TH32CS_SNAPPROCESS: u32 = 0x00000002;
const INVALID_HANDLE_VALUE: isize = -1;

#[repr(C)]
#[allow(non_snake_case)]
struct PROCESSENTRY32W {
    dwSize: u32,
    cntUsage: u32,
    th32ProcessID: u32,
    th32DefaultHeapID: usize,
    th32ModuleID: u32,
    cntThreads: u32,
    th32ParentProcessID: u32,
    pcPriClassBase: i32,
    dwFlags: u32,
    szExeFile: [u16; 260],
}

// ── Focus / window functions ─────────────────────────────────────────

pub fn save_foreground() -> WindowId {
    unsafe { GetForegroundWindow() }
}

pub fn restore_focus(id: WindowId) {
    if id != 0 {
        unsafe { SetForegroundWindow(id); }
    }
}

pub fn is_window_valid(id: WindowId) -> bool {
    id != 0 && unsafe { IsWindow(id) != 0 }
}

pub fn get_focused_window() -> WindowId {
    unsafe { GetForegroundWindow() }
}

// ── Process management ───────────────────────────────────────────────

/// Find claude.exe PID via Toolhelp32 snapshot.
pub fn find_cc_pid() -> Option<u32> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == 0 || snapshot == INVALID_HANDLE_VALUE {
            return None;
        }

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            cntUsage: 0, th32ProcessID: 0, th32DefaultHeapID: 0,
            th32ModuleID: 0, cntThreads: 0, th32ParentProcessID: 0,
            pcPriClassBase: 0, dwFlags: 0, szExeFile: [0u16; 260],
        };

        if Process32FirstW(snapshot, &mut entry) == 0 {
            CloseHandle(snapshot);
            return None;
        }

        let target: Vec<u16> = "claude.exe\0".encode_utf16().collect();
        loop {
            let mut matches = true;
            for (i, &c) in target.iter().enumerate() {
                let ec = entry.szExeFile[i];
                let ec_lower = if (b'A'..=b'Z').contains(&(ec as u8)) { ec | 0x0020 } else { ec };
                let c_lower = if (b'A'..=b'Z').contains(&(c as u8)) { c | 0x0020 } else { c };
                if ec_lower != c_lower as u16 { matches = false; break; }
                if c == 0 { break; }
            }
            if matches {
                let pid = entry.th32ProcessID;
                CloseHandle(snapshot);
                return Some(pid);
            }
            if Process32NextW(snapshot, &mut entry) == 0 {
                break;
            }
        }
        CloseHandle(snapshot);
    }

    // Fallback: PID file (written by SessionStart hook pre-v1.0.5)
    read_cc_pid_file()
}

pub fn is_process_alive(pid: u32) -> bool {
    if pid == 0 { return false; }
    unsafe {
        let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if handle == 0 || handle == -1 {
            return false;
        }
        let result = WaitForSingleObject(handle, 0);
        CloseHandle(handle);
        result == WAIT_TIMEOUT
    }
}

// ── PID file reader (shared) ─────────────────────────────────────────

pub fn read_cc_pid_file() -> Option<u32> {
    let path = std::env::var("TEMP")
        .map(|d| PathBuf::from(d).join("claude-halo-cc-pid.txt"))
        .unwrap_or_else(|_| PathBuf::from("C:\\Windows\\Temp\\claude-halo-cc-pid.txt"));
    let content = fs::read_to_string(&path).ok()?;
    content.trim().parse().ok()
}
