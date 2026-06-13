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
    // Keyboard state polling — works from any thread, any window, no message pump needed
    fn GetAsyncKeyState(vk: i32) -> i16;
    // Focus restoration — save terminal window before halo steals it
    fn GetForegroundWindow() -> isize;
    fn SetForegroundWindow(hWnd: isize) -> i32;
    // Process liveness check — detect when Claude Code has exited
    fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> isize;
    fn CloseHandle(hObject: isize) -> i32;
    fn WaitForSingleObject(hHandle: isize, dwMilliseconds: u32) -> u32;
}

const PROCESS_SYNCHRONIZE: u32 = 0x00100000;
const WAIT_TIMEOUT: u32 = 0x00000102;

// Virtual key codes
const VK_CONTROL: i32 = 0x11;
const VK_SHIFT:   i32 = 0x10;
const VK_F12:     i32 = 0x7B;

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

/// Read the Claude Code PID written by the SessionStart hook.
/// Returns None if the file doesn't exist or is malformed.
fn read_claude_pid() -> Option<u32> {
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

/// Check if Ctrl+Shift+F12 is currently held down.
/// GetAsyncKeyState queries the physical key state at call time.
fn is_hotkey_down() -> bool {
    unsafe {
        let ctrl  = (GetAsyncKeyState(VK_CONTROL) as u16) & 0x8000 != 0;
        let shift = (GetAsyncKeyState(VK_SHIFT)   as u16) & 0x8000 != 0;
        let f12   = (GetAsyncKeyState(VK_F12)     as u16) & 0x8000 != 0;
        ctrl && shift && f12
    }
}

// ── Tauri commands ────────────────────────────────────────────────

#[tauri::command]
async fn get_state(s: tauri::State<'_, AppState>) -> Result<String, String> {
    Ok(s.current_state.lock().await.to_str().to_string())
}

#[tauri::command]
async fn set_passthrough(w: tauri::WebviewWindow, enabled: bool) -> Result<bool, String> {
    w.set_ignore_cursor_events(enabled).map_err(|e| e.to_string())?;
    Ok(enabled)
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
        .invoke_handler(tauri::generate_handler![get_state, set_passthrough])
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
                let mut think_hold_until: Option<std::time::Instant> = None;
                let mut saw_non_executing = true;
                // Process liveness check — detect Claude Code exit without
                // relying on hook execution. SessionStart writes Claude's PID;
                // halo checks every ~15 ticks (~2.25 s).
                let mut alive_check_ticks: u32 = 0; // check on first tick

                // Hotkey: debounced with cooldown to prevent rapid-fire
                // ~2s cooldown = 13 ticks × 150ms
                let mut hotkey_cool: u32 = 0;

                loop {
                    interval.tick().await;

                    // ── Hotkey check (Ctrl+Shift+F12) ────────────
                    if hotkey_cool > 0 {
                        hotkey_cool -= 1;
                    }
                    if hotkey_cool == 0 && is_hotkey_down() {
                        hotkey_cool = 13;
                        let _ = win.emit("toggle-passthrough", ());
                    }

                    // ── Read hook state ──────────────────────────
                    let raw_state = read_hook_state().unwrap_or(HaloState::Idle);

                    // ── Process liveness check (every ~2.25 s) ──
                    // Detect Claude Code exit by checking whether the
                    // claude.exe PID (written by SessionStart) is still alive.
                    // This does NOT rely on hooks firing during shutdown.
                    if alive_check_ticks == 0 {
                        if let Some(pid) = read_claude_pid() {
                            if !is_process_alive(pid) {
                                let _ = win.close();
                                break;
                            }
                        }
                        alive_check_ticks = 15;
                    }
                    alive_check_ticks -= 1;

                    let mut new_state = if matches!(raw_state, HaloState::Completed) && completed_consumed {
                        HaloState::Idle
                    } else {
                        raw_state
                    };

                    // Reset completed_consumed when user starts a new interaction.
                    // Tool-free chats (including /compact) never enter the
                    // Executing/InputNeeded branch that normally clears this flag,
                    // so without this reset the next Completed would be skipped.
                    if !matches!(new_state, HaloState::Idle | HaloState::Completed) && completed_consumed {
                        completed_consumed = false;
                    }

                    // ── Missed-completed injection ─────────────────
                    // idle_prompt notification can overwrite "completed" in the
                    // state file before our 150ms poll catches it, especially in
                    // tool-free chats and after compaction.  If we were showing any
                    // active state and raw_state is now idle/completed, inject Completed.
                    if completed_since.is_none() && !completed_consumed {
                        match (displayed, raw_state) {
                            (Some(HaloState::Thinking | HaloState::Executing | HaloState::Compacting), HaloState::Idle)
                            | (Some(HaloState::Thinking | HaloState::Executing | HaloState::Compacting), HaloState::Completed) => {
                                new_state = HaloState::Completed;
                            }
                            _ => {}
                        }
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
                    if completed_since.is_some() {
                        match new_state {
                            HaloState::Thinking | HaloState::Executing | HaloState::InputNeeded => {
                                // New user interaction — release hold
                                completed_since = None;
                                completed_consumed = false;
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
                        let elapsed = completed_since.unwrap().elapsed();
                        if elapsed.as_secs() >= 3 {
                            completed_consumed = true;
                            completed_since = None;
                            let _ = win.emit("state-changed", HaloState::Idle.to_str());
                            *st.lock().await = HaloState::Idle;
                            displayed = Some(HaloState::Idle);
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
