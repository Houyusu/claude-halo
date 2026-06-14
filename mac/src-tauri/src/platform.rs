// ── macOS platform implementation ────────────────────────────────────
//
// Window-level focus detection via CoreGraphics (CGWindowList).
// Precise CGWindowID matching — same semantics as Windows HWND.
// Focus restore via osascript.  Process liveness via kill(0).
//
// Uses raw CoreFoundation FFI to avoid version-sensitive safe wrappers.

use std::ffi::c_void;
use std::fs;
use std::process::Command;

pub type WindowId = u32;

// ── CoreFoundation / CoreGraphics FFI (manual, stable across versions) ─

type CFIndex = isize;
type CFArrayRef = *const c_void;
type CFDictionaryRef = *const c_void;
type CFStringRef = *const c_void;
type CFNumberRef = *const c_void;
type CFTypeRef = *const c_void;
type CFNumberType = i64;
type Boolean = u8;
type CGWindowID = u32;

const KCG_NUMBER_SINT32: CFNumberType = 3;
const KCG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY: u32 = 0;
const KCG_WINDOW_LIST_OPTION_EXCLUDE_DESKTOP: u32 = 1 << 4;

extern "C" {
    // CGWindowList
    fn CGWindowListCopyWindowInfo(option: u32, relativeToWindow: CGWindowID) -> CFArrayRef;

    // CGWindow dictionary keys (CFStringRef constants)
    static kCGWindowNumber: CFStringRef;
    static kCGWindowOwnerPID: CFStringRef;
    static kCGWindowLayer: CFStringRef;

    // CFArray
    fn CFArrayGetCount(theArray: CFArrayRef) -> CFIndex;
    fn CFArrayGetValueAtIndex(theArray: CFArrayRef, idx: CFIndex) -> *const c_void;

    // CFDictionary
    fn CFDictionaryGetValueIfPresent(
        theDict: CFDictionaryRef,
        key: *const c_void,
        value: *mut *const c_void,
    ) -> Boolean;

    // CFNumber
    fn CFNumberGetValue(
        number: CFNumberRef,
        theType: CFNumberType,
        valuePtr: *mut c_void,
    ) -> Boolean;

    // Memory management
    fn CFRelease(cf: CFTypeRef);
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Extract an i32 value from a CFDictionary for the given key.
fn dict_get_i32(dict: CFDictionaryRef, key: CFStringRef) -> Option<i32> {
    let mut cf_val: *const c_void = std::ptr::null();
    let found = unsafe {
        CFDictionaryGetValueIfPresent(dict, key as *const c_void, &mut cf_val)
    };
    if found == 0 || cf_val.is_null() {
        return None;
    }
    let mut val: i32 = 0;
    let ok = unsafe { CFNumberGetValue(cf_val as CFNumberRef, KCG_NUMBER_SINT32, &mut val as *mut i32 as *mut c_void) };
    if ok == 0 { None } else { Some(val) }
}

/// Return the first on-screen (layer 0) window from CGWindowList.
/// The list is ordered front-to-back, so the first match is the frontmost window.
fn get_frontmost_window() -> Option<CGWindowID> {
    let opts = KCG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY | KCG_WINDOW_LIST_OPTION_EXCLUDE_DESKTOP;
    let array_ref = unsafe { CGWindowListCopyWindowInfo(opts, 0) };
    if array_ref.is_null() {
        return None;
    }
    let count = unsafe { CFArrayGetCount(array_ref) };

    for i in 0..count {
        let dict_ref = unsafe { CFArrayGetValueAtIndex(array_ref, i) } as CFDictionaryRef;

        // Skip system overlay windows (layer ≠ 0)
        if let Some(layer) = dict_get_i32(dict_ref, unsafe { kCGWindowLayer }) {
            if layer != 0 {
                continue;
            }
        }
        if let Some(num) = dict_get_i32(dict_ref, unsafe { kCGWindowNumber }) {
            unsafe { CFRelease(array_ref as CFTypeRef); }
            return Some(num as CGWindowID);
        }
    }
    unsafe { CFRelease(array_ref as CFTypeRef); }
    None
}

/// Check whether a CGWindowID still exists in the on-screen window list.
fn window_id_exists(window_id: CGWindowID) -> bool {
    let opts = KCG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY | KCG_WINDOW_LIST_OPTION_EXCLUDE_DESKTOP;
    let array_ref = unsafe { CGWindowListCopyWindowInfo(opts, 0) };
    if array_ref.is_null() {
        return false;
    }
    let count = unsafe { CFArrayGetCount(array_ref) };

    let mut exists = false;
    for i in 0..count {
        let dict_ref = unsafe { CFArrayGetValueAtIndex(array_ref, i) } as CFDictionaryRef;
        if let Some(num) = dict_get_i32(dict_ref, unsafe { kCGWindowNumber }) {
            if num as CGWindowID == window_id {
                exists = true;
                break;
            }
        }
    }
    unsafe { CFRelease(array_ref as CFTypeRef); }
    exists
}

// ── Focus / window functions ─────────────────────────────────────────

pub fn save_foreground() -> WindowId {
    get_frontmost_window().unwrap_or(0)
}

pub fn restore_focus(_id: WindowId) {
    // osascript is the simplest way to refocus the previous app.
    // On macOS the halo window rarely steals focus, so this is belt-and-suspenders.
    let script = format!(
        "tell application \"System Events\" to set frontmost of first process \
         whose unix id is {} to true",
        _id
    );
    let _ = Command::new("osascript")
        .args(["-e", &script])
        .output();
}

pub fn is_window_valid(id: WindowId) -> bool {
    id != 0 && window_id_exists(id as CGWindowID)
}

pub fn get_focused_window() -> WindowId {
    get_frontmost_window().unwrap_or(0)
}

/// Get the process ID that owns this window.
/// Looks up the CGWindowID in the on-screen window list.
pub fn get_window_pid(id: WindowId) -> Option<u32> {
    if id == 0 {
        return None;
    }
    let opts = KCG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY | KCG_WINDOW_LIST_OPTION_EXCLUDE_DESKTOP;
    let array_ref = unsafe { CGWindowListCopyWindowInfo(opts, 0) };
    if array_ref.is_null() {
        return None;
    }
    let count = unsafe { CFArrayGetCount(array_ref) };
    let mut pid: Option<u32> = None;
    for i in 0..count {
        let dict_ref = unsafe { CFArrayGetValueAtIndex(array_ref, i) } as CFDictionaryRef;
        if let Some(num) = dict_get_i32(dict_ref, unsafe { kCGWindowNumber }) {
            if num as CGWindowID == id {
                pid = dict_get_i32(dict_ref, unsafe { kCGWindowOwnerPID }).map(|p| p as u32);
                break;
            }
        }
    }
    unsafe { CFRelease(array_ref as CFTypeRef); }
    pid
}

// ── Process management ───────────────────────────────────────────────

/// Find the Claude Code process PID.
/// Prefers the PID file written by launch-halo.sh (fast, exact).
/// Falls back to `pgrep` (slower, scans all processes).
pub fn find_cc_pid() -> Option<u32> {
    // 1. PID file (written by launch-halo.sh via $PPID)
    if let Some(pid) = read_cc_pid_file() {
        if is_process_alive(pid) {
            return Some(pid);
        }
    }

    // 2. pgrep fallback
    if let Ok(output) = Command::new("pgrep")
        .args(["-f", "claude"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = stdout.lines().next() {
            if let Ok(pid) = line.trim().parse::<u32>() {
                return Some(pid);
            }
        }
    }

    None
}

/// Check whether a process is alive using POSIX kill(0).
/// Returns true if the process exists, false otherwise.
pub fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    // kill(pid, 0) — no signal sent, only permissions/ESRCH checked.
    // Returns 0 if the process exists, -1 with errno=ESRCH if not.
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

// ── PID file reader ──────────────────────────────────────────────────

fn read_cc_pid_file() -> Option<u32> {
    let path = std::env::temp_dir().join("claude-halo-cc-pid.txt");
    let content = fs::read_to_string(&path).ok()?;
    content.trim().parse().ok()
}
