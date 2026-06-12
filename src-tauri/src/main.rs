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
    // Foreground window detection
    fn GetForegroundWindow() -> isize;
    fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> isize;
    fn CloseHandle(h: isize) -> i32;
    fn QueryFullProcessImageNameW(h: isize, flags: u32, buf: *mut u16, len: *mut u32) -> i32;

    // Keyboard state polling — works from any thread, any window, no message pump needed
    fn GetAsyncKeyState(vk: i32) -> i16;
}

const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

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

fn is_user_glancing_at_claude() -> bool {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == 0 { return false; }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 { return false; }

        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle == 0 { return false; }

        let mut buf: [u16; 260] = [0; 260];
        let mut len: u32 = 260;
        let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut len);
        CloseHandle(handle);

        if ok == 0 { return false; }

        let path = String::from_utf16_lossy(&buf[..len as usize]).to_lowercase();
        if let Some(name) = path.rsplit('\\').next() {
            name == "pwsh.exe" || name == "powershell.exe"
                || name == "windowsterminal.exe" || name == "wt.exe"
                || name == "cmd.exe" || name == "conhost.exe"
        } else {
            false
        }
    }
}

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

fn read_heartbeat_age() -> Option<std::time::Duration> {
    let path = std::env::var("TEMP")
        .map(|d| PathBuf::from(d).join("claude-halo-heartbeat.txt"))
        .unwrap_or_else(|_| PathBuf::from("C:\\Windows\\Temp\\claude-halo-heartbeat.txt"));

    let meta = fs::metadata(&path).ok()?;
    let mtime = meta.modified().ok()?;
    mtime.elapsed().ok()
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
async fn set_halo_size(w: tauri::WebviewWindow, size: f64) -> Result<f64, String> {
    w.set_size(tauri::LogicalSize::new(size, size)).map_err(|e| e.to_string())?;
    Ok(size)
}

#[tauri::command]
async fn set_passthrough(w: tauri::WebviewWindow, enabled: bool) -> Result<bool, String> {
    w.set_ignore_cursor_events(enabled).map_err(|e| e.to_string())?;
    Ok(enabled)
}

// ── Main ──────────────────────────────────────────────────────────

fn main() {
    let state = Arc::new(Mutex::new(HaloState::Idle));
    let state_clone = state.clone();

    tauri::Builder::default()
        .manage(AppState { current_state: state })
        .invoke_handler(tauri::generate_handler![get_state, set_passthrough, set_halo_size])
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
                let mut ticks: u64 = 0;

                // Hotkey: debounced with cooldown to prevent rapid-fire
                // ~2s cooldown = 13 ticks × 150ms
                let mut hotkey_cool: u32 = 0;

                loop {
                    interval.tick().await;
                    ticks += 1;

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

                    // ── Heartbeat check (every ~3s) ──────────────
                    if ticks % 20 == 0 {
                        if let Some(age) = read_heartbeat_age() {
                            if age.as_secs() >= 7 {
                                // Heartbeat stale → Claude Code is gone
                                let _ = win.close();
                                break;
                            }
                        }
                        // No heartbeat file = manual launch, don't auto-close
                    }

                    let mut new_state = if matches!(raw_state, HaloState::Completed) && completed_consumed {
                        HaloState::Idle
                    } else {
                        raw_state
                    };

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
                        if std::time::Instant::now() < deadline {
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
                    if let Some(t) = exec_since {
                        if t.elapsed().as_millis() < 1500 { continue; }
                        exec_since = None;
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
                        if elapsed.as_secs() >= 3 && is_user_glancing_at_claude() {
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
