#!/bin/bash
# Claude Halo — state file writer (bash, macOS / Linux / Git-Bash on Windows)
# Atomic write without BOM.  Functionally identical to halo-hook.ps1.
set -euo pipefail

STATE="${1:-idle}"

# Platform-appropriate temp directory
if [ "$(uname -s)" = "Darwin" ]; then
    # macOS: TMPDIR always set (e.g. /var/folders/.../T/)
    STATE_DIR="${TMPDIR%/}"
elif [ -n "${TEMP:-}" ]; then
    # Windows (Git Bash / MSYS2): TEMP is set
    STATE_DIR="$TEMP"
else
    # Linux / fallback
    STATE_DIR="${TMPDIR:-/tmp}"
fi

STATE_FILE="$STATE_DIR/claude-halo-state.txt"

# If compacting just finished, show completed (green) instead of idle.
# SessionStart fires right after PreCompact — the user wants a clear
# signal that compacting is done.
if [ "$STATE" = "idle" ] && [ -f "$STATE_FILE" ]; then
    PREV=$(cat "$STATE_FILE" 2>/dev/null || true)
    if [ "$PREV" = "compacting" ]; then
        STATE="completed"
    fi
fi

# Atomic write: write to temp, then mv (same-volume mv is atomic on all platforms)
TMP_FILE="$STATE_FILE.tmp"
printf '%s' "$STATE" > "$TMP_FILE"
mv "$TMP_FILE" "$STATE_FILE"

exit 0
