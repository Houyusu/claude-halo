// ── macOS platform implementation ────────────────────────────────────
//
// Window-level focus detection via CoreGraphics (CGWindowList).
// Precise CGWindowID matching — same semantics as Windows HWND.
// Focus restore via osascript.  Process liveness via kill(0).

use std::ffi::c_void;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use core_foundation::array::CFArray;
use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex};
use core_foundation_sys::base::CFTypeRef;
use core_foundation_sys::dictionary::{CFDictionaryGetValueIfPresent, CFDictionaryRef};
use core_foundation_sys::number::{kCFNumberSInt32Type, CFNumberGetValue};
use core_graphics::window::{
    CGWindowListCopyWindowInfo, kCGWindowListExcludeDesktopElements,
    kCGWindowListOptionOnScreenOnly,
};

pub type WindowId = u32;

// ── CGWindow dictionary key constants ────────────────────────────────

extern "C" {
    // These are CFStringRef globals defined by CoreGraphics/CGWindow.h.
    // Declared as *const c_void to avoid depending on the exact opaque
    // CFString type, which varies across core-foundation versions.
    static kCGWindowNumber: *const c_void;
    static kCGWindowOwnerPID: *const c_void;
    static kCGWindowLayer: *const c_void;
}

fn cfstr_from_static(r: &'static *const c_void) -> CFString {
    unsafe { CFString::wrap_under_get_rule(*r as *const _) }
}

/// Get an i32 value from a CFDictionary for a given key.
fn dict_get_i32(dict: CFDictionaryRef, key: &CFString) -> Option<i32> {
    let mut cf_val: *const c_void = std::ptr::null();
    let found = unsafe {
        CFDictionaryGetValueIfPresent(dict, key.as_void_ptr(), &mut cf_val)
    };
    if found == 0 || cf_val.is_null() {
        return None;
    }
    let mut val: i32 = 0;
    let ok = unsafe { CFNumberGetValue(cf_val as _, kCFNumberSInt32Type, &mut val) };
    if ok != 0 { Some(val) } else { None }
}

/// Return the first on-screen, non-system-layer window from CGWindowList.
/// The list is ordered front-to-back, so the first match is the frontmost.
fn get_frontmost_window() -> Option<u32> {
    let opts = kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements;
    let array_ref = unsafe { CGWindowListCopyWindowInfo(opts, 0) };
    if array_ref.is_null() {
        return None;
    }
    let array = unsafe { CFArray::wrap_under_create_rule(array_ref as *mut _) };
    let count = unsafe { CFArrayGetCount(array.as_CFTypeRef() as _) };
    let layer_key = cfstr_from_static(unsafe { &kCGWindowLayer });
    let number_key = cfstr_from_static(unsafe { &kCGWindowNumber });

    for i in 0..count {
        let dict_ref = unsafe { CFArrayGetValueAtIndex(array.as_CFTypeRef() as _, i) } as CFDictionaryRef;
        // Skip system overlay windows (layer ≠ 0)
        if let Some(layer) = dict_get_i32(dict_ref, &layer_key) {
            if layer != 0 {
                continue;
            }
        }
        if let Some(num) = dict_get_i32(dict_ref, &number_key) {
            return Some(num as u32);
        }
    }
    None
}

/// Check whether a CGWindowID still exists in the on-screen window list.
fn window_id_exists(window_id: u32) -> bool {
    let opts = kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements;
    let array_ref = unsafe { CGWindowListCopyWindowInfo(opts, 0) };
    if array_ref.is_null() {
        return false;
    }
    let array = unsafe { CFArray::wrap_under_create_rule(array_ref as *mut _) };
    let count = unsafe { CFArrayGetCount(array.as_CFTypeRef() as _) };
    let number_key = cfstr_from_static(unsafe { &kCGWindowNumber });

    for i in 0..count {
        let dict_ref =
            unsafe { CFArrayGetValueAtIndex(array.as_CFTypeRef() as _, i) } as CFDictionaryRef;
        if let Some(num) = dict_get_i32(dict_ref, &number_key) {
            if num as u32 == window_id {
                return true;
            }
        }
    }
    false
}

// ── Focus / window functions ─────────────────────────────────────────

pub fn save_foreground() -> WindowId {
    // On macOS the transparent halo window typically does not steal focus,
    // but we save the frontmost window ID anyway for consistency.
    get_frontmost_window().unwrap_or(0)
}

pub fn restore_focus(_id: WindowId) {
    // osascript is the simplest way to refocus the previous app.
    // We use `ignoring application responses` so the script doesn't block.
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
    id != 0 && window_id_exists(id)
}

pub fn get_focused_window() -> WindowId {
    get_frontmost_window().unwrap_or(0)
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
    // This is the canonical POSIX process-liveness check.
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

// ── PID file reader ──────────────────────────────────────────────────

fn read_cc_pid_file() -> Option<u32> {
    let path = std::env::temp_dir().join("claude-halo-cc-pid.txt");
    let content = fs::read_to_string(&path).ok()?;
    content.trim().parse().ok()
}
