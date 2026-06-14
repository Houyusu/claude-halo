mod platform;

use serde::Serialize;
use std::fs;
use std::sync::Arc;
use tauri::Emitter;
use tauri::webview::WebviewWindowBuilder;
use tokio::sync::Mutex;

// ── Halo state ───────────────────────────────────────────────────────

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

// ── State file reader (cross-platform) ───────────────────────────────

fn read_hook_state() -> Option<HaloState> {
    let path = std::env::temp_dir().join("claude-halo-state.txt");
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

// ── Tauri commands ───────────────────────────────────────────────────

#[tauri::command]
async fn get_state(s: tauri::State<'_, AppState>) -> Result<String, String> {
    Ok(s.current_state.lock().await.to_str().to_string())
}

// ── Main ─────────────────────────────────────────────────────────────

fn main() {
    // Save foreground window BEFORE tauri creates the halo window.
    let saved_hwnd = platform::save_foreground();

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
            if saved_hwnd != platform::WindowId::default() {
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
                    platform::restore_focus(saved_hwnd);
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
                let mut focus_hwnd = saved_hwnd; // terminal window handle for focus detection
                let mut focus_captured = false;   // captured on first Thinking
                let mut think_hold_until: Option<std::time::Instant> = None;
                let mut saw_non_executing = true;
                // Process liveness check: find claude process and poll liveness.
                let mut alive_check_ticks: u32 = 0; // check on first tick
                let mut cc_pid_cache: Option<u32> = None; // cached CC PID for focus verification

                loop {
                    interval.tick().await;

                    // ── Read hook state ──────────────────────────
                    let raw_state = read_hook_state().unwrap_or(HaloState::Idle);

                    // ── Process liveness check (every ~2.25 s) ──
                    if alive_check_ticks == 0 {
                        if let Some(pid) = platform::find_cc_pid() {
                            if !platform::is_process_alive(pid) {
                                let _ = win.close();
                                break;
                            }
                            cc_pid_cache = Some(pid);
                        } else {
                            // No CC process running at all — exit
                            let _ = win.close();
                            break;
                        }
                        alive_check_ticks = 15;
                    }
                    alive_check_ticks -= 1;

                    // Capture terminal window ID on the first Thinking transition.
                    if focus_captured && !platform::is_window_valid(focus_hwnd) {
                        focus_captured = false;
                    }
                    if !focus_captured && matches!(raw_state, HaloState::Thinking) {
                        let fg = platform::get_focused_window();
                        if fg != platform::WindowId::default() {
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
                    // starts a new interaction.
                    if !matches!(new_state, HaloState::Idle | HaloState::Completed) && completed_consumed {
                        completed_consumed = false;
                    }
                    if !matches!(new_state, HaloState::Idle | HaloState::Completed) && hold_completed {
                        hold_completed = false;
                    }

                    // ── Missed-completed injection ─────────────────
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
                    if let Some(t) = exec_since {
                        if t.elapsed().as_millis() < 1500 && !matches!(new_state, HaloState::Completed) {
                            continue;
                        }
                        exec_since = None;
                    }

                    // Completed hold
                    if completed_since.is_some() {
                        match new_state {
                            HaloState::Thinking | HaloState::Executing | HaloState::InputNeeded | HaloState::Compacting => {
                                completed_since = None;
                                completed_consumed = false;
                                hold_completed = false;
                            }
                            _ => {
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
                        let fg = platform::get_focused_window();
                        let terminal_focused = fg != platform::WindowId::default() && fg == focus_hwnd;
                        let focus_valid = platform::is_window_valid(focus_hwnd);
                        let effectively_focused = if terminal_focused {
                            true
                        } else if !focus_valid && fg != platform::WindowId::default() {
                            cc_pid_cache.map_or(!hold_completed, |cc_pid| {
                                platform::get_window_pid(fg) == Some(cc_pid)
                            })
                        } else {
                            false
                        };
                        if !hold_completed && !effectively_focused {
                            hold_completed = true;
                        }
                        if hold_completed && effectively_focused {
                            if !focus_valid {
                                focus_hwnd = fg;
                                focus_captured = true;
                            }
                            hold_completed = false;
                            completed_consumed = true;
                            completed_since = None;
                            let _ = win.emit("state-changed", HaloState::Idle.to_str());
                            *st.lock().await = HaloState::Idle;
                            displayed = Some(HaloState::Idle);
                            continue;
                        }
                        // Normal completed (user watching): hold for 3s, then fade.
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
