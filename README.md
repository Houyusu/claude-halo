# Claude Halo

A Tauri v2 desktop overlay that displays a morphing status ring reflecting Claude Code's session state.

## States

| State | Color | Description |
|---|---|---|
| Idle | `#aaaaaa` | Waiting for input |
| Thinking | `#ff8830` | Claude is reasoning |
| Executing | `#3399ff` | Running tools |
| Input Needed | `#ee3333` | Awaiting user response |
| Completed | `#33cc55` | Task finished |
| Compacting | `#9944ff` | Context compaction |

## How It Works

- **Auto-start**: Launched by Claude Code's `SessionStart` hook
- **Heartbeat**: Refreshed every 5s via statusline hook; goes stale after 7s → auto-close
- **Auto-close**: `Stop` hook sends `taskkill` on normal exit; heartbeat timeout catches crashes

## Build

```bash
cd src-tauri
cargo build --release
```

Output: `src-tauri/target/release/claude-halo.exe`

## Tech Stack

- [Tauri v2](https://v2.tauri.app/) — Rust backend + webview frontend
- Canvas 2D rendering — custom ring animation with morphing, glow bridge, and radius pulse
- Win32 FFI — foreground window detection and global hotkey polling via `GetAsyncKeyState`
