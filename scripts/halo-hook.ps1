param([string]$State = "idle", [switch]$NoHeartbeat)

# Atomic write without BOM: use .NET for UTF-8 without BOM directly
$stateFile = Join-Path $env:TEMP "claude-halo-state.txt"
$tempFile   = "$stateFile.tmp"

try {
    [System.IO.File]::WriteAllText($tempFile, $State, [System.Text.UTF8Encoding]::new($false))
    Move-Item -Force $tempFile $stateFile
} catch {
    # Best-effort fallback
    [System.IO.File]::WriteAllText($stateFile, $State, [System.Text.UTF8Encoding]::new($false))
}

# Touch heartbeat so halo knows Claude Code is still alive.
# Skip for "completed" — the Stop hook deletes the heartbeat after this
# call, so touching it here would defeat the exit signal.
if (-not $NoHeartbeat -and $State -ne "completed") {
    try {
        $hb = Join-Path $env:TEMP "claude-halo-heartbeat.txt"
        $now = (Get-Date).ToUniversalTime().ToString("o")
        [System.IO.File]::WriteAllText($hb, $now, [System.Text.UTF8Encoding]::new($false))
    } catch {}
}

exit 0
