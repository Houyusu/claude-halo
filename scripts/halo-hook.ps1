param([string]$State = "idle")

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

# Touch heartbeat so halo knows Claude Code is still alive
try {
    $hb = Join-Path $env:TEMP "claude-halo-heartbeat.txt"
    $now = (Get-Date).ToUniversalTime().ToString("o")
    [System.IO.File]::WriteAllText($hb, $now, [System.Text.UTF8Encoding]::new($false))
} catch {}

exit 0
