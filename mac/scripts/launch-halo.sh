#!/bin/bash
# Claude Halo — launcher (bash, macOS / Linux / Git-Bash on Windows)
# SessionStart helper: launch halo if not already running.
#
# IMPORTANT: SessionStart fires on EVERY turn (including after PreCompact),
# so we must NOT kill+restart halo — that would reset its animation state
# and prevent the "compacting → completed → idle" transition from showing.
set -euo pipefail

# Check if halo is already running
if pgrep -f "claude-halo" > /dev/null 2>&1; then
    exit 0
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
HALO_BIN="$SCRIPT_DIR/../bin/claude-halo"

if [ ! -f "$HALO_BIN" ]; then
    exit 0
fi

# macOS: the halo binary is inside the app bundle at
# claude-halo.app/Contents/MacOS/claude-halo
if [ "$(uname -s)" = "Darwin" ] && [ -d "$HALO_BIN.app" ]; then
    open -g "$HALO_BIN.app"
else
    "$HALO_BIN" &
fi

# Write CC PID file for process liveness detection.
# Hook scripts are forked by Claude Code, so $PPID is CC's PID.
if [ "$(uname -s)" = "Darwin" ]; then
    STATE_DIR="${TMPDIR%/}"
elif [ -n "${TEMP:-}" ]; then
    STATE_DIR="$TEMP"
else
    STATE_DIR="${TMPDIR:-/tmp}"
fi
echo "$PPID" > "$STATE_DIR/claude-halo-cc-pid.txt"

exit 0
