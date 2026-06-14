#![windows_subsystem = "windows"]

use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Emitter;
use tauri::webview::WebviewWindowBuilder;
use tokio::sync::Mutex;

// ── Win32 FFI (all in-process, no subprocess spawns) ──────────────
extern "system" {
    // Focus restoration — save terminal window before halo steals it
    fn GetForegroundWindow() -> isize;
    fn SetForegroundWindow(hWnd: isize) -> i32;
    // IsWindow — check if a window handle is still valid
    fn IsWindow(hWnd: isize) -> i32;
    // Process liveness check — detect when Claude Code has exited
    fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> isize;
    fn CloseHandle(hObject: isize) -> i32;
    fn WaitForSingleObject(hHandle: isize, dwMilliseconds: u32) -> u32;
    // Toolhelp32 — enumerate running processes by name
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

// ── Halo state ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
enum HaloState { Idle, Thinking, Executing, InputNeeded, Completed, Compacting }

impl HaloState {
    fn to_str(&self) -> &'static str {
        match self {
            HaloState::Idle => "idle",
            HaloState::Thinking => "thinking",
            HaloState::Executing => "executing",
            HaloState::InputNeeded => "input_needed",
            HaloState::Completed => "completed",
            HaloState::Compacting => "compacting",
        }
    }
}

struct AppState { current_state: Arc<Mutex<HaloState>> }

// ── Win32 helpers ─────────────────────────────────────────────────

fn read_hook_state() -> Option<HaloState> {
    let path = std::env::var("TEMP")
        .map(|d| PathBuf::from(d).join("claude-halo-state.txt"))
        .unwrap_or_else(|_| PathBuf::from("C:\\Windows\\Temp\\claude-halo-state.txt"));

    let content = fs::read_to_string(&path).ok()?;
    let trimmed = content.trim().trim_start_matches('\u{feff}');
    Some(match trimmed {
        "thinking"     => HaloState::Thinking,
        "executing"    => HaloState::Executing,
        "input_needed" => HaloState::InputNeeded,
        "completed"    => HaloState::Completed,
        "compacting"   => HaloState::Compacting,
        _              => HaloState::Idle,
    })
}

/// Find the PID of a running claude.exe process using Toolhelp32.
/// Because we are launched by Claude Code, there is always at least one
/// claude.exe running when halo starts. No hook or file I/O needed.
fn find_claude_pid() -> Option<u32> {
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
            // Compare szExeFile (UTF-16LE, case-insensitive)
            let mut matches = true;
            for (i, &c) in target.iter().enumerate() {
                let ec = entry.szExeFile[i];
                // Case-insensitive ASCII comparison for Latin letters
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

    // Fallback: try reading PID file (written by SessionStart hook pre-v1.0.5)
    read_cc_pid_file()
}

/// Legacy fallback: read PID from file for backward compatibility.
fn read_cc_pid_file() -> Option<u32> {
    let path = std::env::var("TEMP")
        .map(|d| PathBuf::from(d).join("claude-halo-cc-pid.txt"))
        .unwrap_or_else(|_| PathBuf::from("C:\\Windows\\Temp\\claude-halo-cc-pid.txt"));
    let content = fs::read_to_string(&path).ok()?;
    content.trim().parse().ok()
}

/// Check if a Windows process is still alive using a SYNCHRONIZE handle.
/// OpenProcess + WaitForSingleObject(timeout=0) is the canonical check:
///   WAIT_TIMEOUT → process still running
///   anything else → process has exited (or OpenProcess failed)
fn is_process_alive(pid: u32) -> bool {
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

// ── Tauri commands ────────────────────────────────────────────────

#[tauri::command]
async fn get_state(s: tauri::State<'_, AppState>) -> Result<String, String> {
    Ok(s.current_state.lock().await.to_str().to_string())
}

// ── Main ──────────────────────────────────────────────────────────

fn main() {
    // Save foreground window BEFORE tauri creates the halo window.
    // Without this, the terminal loses focus every time halo starts.
    let saved_hwnd = unsafe { GetForegroundWindow() };

    let state = Arc::new(Mutex::new(HaloState::Idle));
    let state_clone = state.clone();

    tauri::Builder::default()
        .manage(AppState { current_state: state })
        .invoke_handler(tauri::generate_handler![get_state])
        .setup(move |app| {
            let window = WebviewWindowBuilder::new(app, "main",
                tauri::WebviewUrl::App("index.html".into())
            )
            .title("Claude Halo")
            .inner_size(100.0, 100.0)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .shadow(false)
            .initialization_script("document.documentElement.style.setProperty('background','transparent','important');document.body.style.setProperty('background','transparent','important');")
            .build()?;

            // Mouse passthrough — halo is a pure visual indicator,
            // all clicks pass through to windows beneath.
            let _ = window.set_ignore_cursor_events(true);

            // Position at bottom-right of primary monitor
            // x: -28px from right edge, y: -140px from bottom
            if let Ok(Some(monitor)) = window.primary_monitor() {
                let m = monitor.size();
                let ws = window.outer_size().unwrap();
                let x = (m.width as i32 - ws.width as i32 - 28).max(0);
                let y = (m.height as i32 - ws.height as i32 - 140).max(0);
                let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(x, y)));
            }

            // Restore focus to the terminal — halo's window steals it on creation
            if saved_hwnd != 0 {
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
                    unsafe { SetForegroundWindow(saved_hwnd); }
                });
            }

            let win = window.clone();
            let st = state_clone;

            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(150));
                let mut displayed: Option<HaloState> = None;
                let mut exec_since: Option<std::time::Instant> = None;
                let mut completed_since: Option<std::time::Instant> = None;
                let mut completed_consumed = false;
                let mut hold_completed = false; // user-away: hold green until focus returns
                let mut focus_hwnd = saved_hwnd; // terminal handle for focus detection
                let mut focus_captured = false;   // captured on first Thinking
                let mut think_hold_until: Option<std::time::Instant> = None;
                let mut saw_non_executing = true;
                // Process liveness check: find claude.exe via Toolhelp32
                // enumeration and poll its liveness. No hook or file needed.
                let mut alive_check_ticks: u32 = 0; // check on first tick

                // Hotkey: debounced with cooldown to prevent rapid-fire
                // ~2s cooldown = 13 ticks × 150ms
                loop {
                    interval.tick().await;

                    // ── Read hook state ──────────────────────────
                    let raw_state = read_hook_state().unwrap_or(HaloState::Idle);

                    // ── Process liveness check (every ~2.25 s) ──
                    // Halo enumerates processes to find claude.exe
                    // (Toolhelp32). No hook execution needed.
                    if alive_check_ticks == 0 {
                        if let Some(pid) = find_claude_pid() {
                            if !is_process_alive(pid) {
                                let _ = win.close();
                                break;
                            }
                        } else {
                            // No claude.exe running at all — exit
                            let _ = win.close();
                            break;
                        }
                        alive_check_ticks = 15;
                    }
                    alive_check_ticks -= 1;

                    // Capture terminal HWND on the first Thinking transition.
                    // At the exact moment "thinking" fires, the user just hit
                    // Enter in their terminal — GetForegroundWindow() is guaranteed
                    // to be the terminal window.  We capture once and never update
                    // again, because during long executions the user may switch away.
                    // Reset if the captured handle becomes invalid (terminal restart).
                    if focus_captured && unsafe { IsWindow(focus_hwnd) == 0 } {
                        focus_captured = false;
                    }
                    if !focus_captured && matches!(raw_state, HaloState::Thinking) {
                        let fg = unsafe { GetForegroundWindow() };
                        if fg != 0 {
                            focus_hwnd = fg;
                            focus_captured = true;
                        }
                    }

                    let mut new_state = if matches!(raw_state, HaloState::Completed) && completed_consumed {
                        HaloState::Idle
                    } else {
                        raw_state
                    };

                    // Reset completed_consumed / hold_completed when user
                    // starts a new interaction.  Tool-free chats (including /compact)
                    // never enter the Executing/InputNeeded branch that normally clears
                    // these flags, so without this reset the next Completed would be skipped.
                    if !matches!(new_state, HaloState::Idle | HaloState::Completed) && completed_consumed {
                        completed_consumed = false;
                    }
                    if !matches!(new_state, HaloState::Idle | HaloState::Completed) && hold_completed {
                        hold_completed = false;
                    }

                    // ── Missed-completed injection ─────────────────
                    // idle_prompt notification can overwrite "completed" in the
                    // state file before our 150ms poll catches it, especially in
                    // tool-free chats and after compaction.  If we were showing any
                    // active state and raw_state is now idle/completed, inject Completed.
                    if completed_since.is_none() && !completed_consumed {
                        match (displayed, raw_state) {
                            (Some(HaloState::Thinking | HaloState::Executing | HaloState::InputNeeded | HaloState::Compacting), HaloState::Idle)
                            | (Some(HaloState::Thinking | HaloState::Executing | HaloState::InputNeeded | HaloState::Compacting), HaloState::Completed) => {
                                new_state = HaloState::Completed;
                            }
                            _ => {}
                        }
                    }

                    // Hold completed when user is away (terminal lost focus at completion).
                    // idle_prompt writes "idle" to the state file, but we keep showing
                    // green until the user returns focus to the terminal.
                    if hold_completed && matches!(raw_state, HaloState::Idle) {
                        new_state = HaloState::Completed;
                    }

                    // Thinking hold (1200ms minimum amber)
                    if matches!(new_state, HaloState::Executing)
                        && saw_non_executing
                        && think_hold_until.is_none()
                    {
                        think_hold_until = Some(std::time::Instant::now()
                            + std::time::Duration::from_millis(1200));
                        new_state = HaloState::Thinking;
                        saw_non_executing = false;
                    }

                    if !matches!(new_state, HaloState::Executing)
                        && !matches!(raw_state, HaloState::Executing)
                    {
                        saw_non_executing = true;
                    }

                    if let Some(deadline) = think_hold_until {
                        if std::time::Instant::now() < deadline
                            && !matches!(new_state, HaloState::Completed)
                        {
                            new_state = HaloState::Thinking;
                        } else {
                            think_hold_until = None;
                        }
                    }

                    // Executing / InputNeeded (highest priority)
                    if matches!(new_state, HaloState::Executing | HaloState::InputNeeded) {
                        if matches!(new_state, HaloState::Executing) && exec_since.is_none() {
                            exec_since = Some(std::time::Instant::now());
                        }
                        completed_since = None;
                        completed_consumed = false;
                        if displayed != Some(new_state) {
                            let s = new_state.to_str();
                            let _ = win.emit("state-changed", s);
                            *st.lock().await = new_state;
                            displayed = Some(new_state);
                        }
                        continue;
                    }

                    // Executing minimum hold (1.5s)
                    // Completed bypass: fast tool calls (<1.5s) still need to show green
                    if let Some(t) = exec_since {
                        if t.elapsed().as_millis() < 1500 && !matches!(new_state, HaloState::Completed) {
                            continue;
                        }
                        exec_since = None;
                    }

                    // Completed hold: once we start showing completed, lock it until
                    // the 3s display condition is met — even if idle_prompt
                    // notification overwrites the state file back to "idle".
                    // Exception: hold_completed holds indefinitely until user
                    // interacts — but a new message or compaction releases it.
                    if completed_since.is_some() {
                        match new_state {
                            HaloState::Thinking | HaloState::Executing | HaloState::InputNeeded | HaloState::Compacting => {
                                // New user interaction or re-compaction — release hold
                                completed_since = None;
                                completed_consumed = false;
                                hold_completed = false;
                            }
                            _ => {
                                // Keep showing completed
                                new_state = HaloState::Completed;
                            }
                        }
                    }

                    // Completed
                    if matches!(new_state, HaloState::Completed) {
                        if completed_since.is_none() {
                            completed_since = Some(std::time::Instant::now());
                            if displayed != Some(HaloState::Completed) {
                                let _ = win.emit("state-changed", HaloState::Completed.to_str());
                                *st.lock().await = HaloState::Completed;
                                displayed = Some(HaloState::Completed);
                            }
                        }
                        // Monitor focus continuously during the completed display.
                        // If the terminal loses focus, the user may have walked away
                        // — hold green indefinitely.  When focus returns, fade to idle.
                        let fg = unsafe { GetForegroundWindow() };
                        let terminal_focused = fg != 0 && fg == focus_hwnd;
                        let focus_valid = unsafe { IsWindow(focus_hwnd) != 0 };
                        // Fallback: if terminal HWND is stale (e.g. after compaction
                        // restart), treat any foreground window as "focused" so the
                        // 3s timer can run.  Without this, a stale HWND would cause
                        // hold_completed to be set indefinitely.
                        let effectively_focused = terminal_focused || (!focus_valid && fg != 0);
                        if !hold_completed && !effectively_focused {
                            hold_completed = true;
                        }
                        if hold_completed && effectively_focused {
                            // User returned — release hold and fade to idle
                            hold_completed = false;
                            completed_consumed = true;
                            completed_since = None;
                            let _ = win.emit("state-changed", HaloState::Idle.to_str());
                            *st.lock().await = HaloState::Idle;
                            displayed = Some(HaloState::Idle);
                            continue;
                        }
                        // Normal completed (user watching): hold for 3s, then fade.
                        // hold_completed (user-away): hold until focus returns.
                        if !hold_completed {
                            let elapsed = completed_since.unwrap().elapsed();
                            if elapsed.as_secs() >= 3 {
                                completed_consumed = true;
                                completed_since = None;
                                let _ = win.emit("state-changed", HaloState::Idle.to_str());
                                *st.lock().await = HaloState::Idle;
                                displayed = Some(HaloState::Idle);
                            }
                        }
                        continue;
                    }

                    // Thinking / Idle
                    if displayed != Some(new_state) {
                        let s = new_state.to_str();
                        let _ = win.emit("state-changed", s);
                        *st.lock().await = new_state;
                        displayed = Some(new_state);
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error");
}
