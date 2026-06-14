param([string]$State = "idle")

# Atomic write without BOM: use .NET for UTF-8 without BOM directly
$stateFile = Join-Path $env:TEMP "claude-halo-state.txt"
$tempFile   = "$stateFile.tmp"

# If compacting just finished, show completed (green) instead of idle (gray).
# SessionStart fires right after PreCompact, and the user wants a clear
# signal that compacting is done — especially since it takes a while.
if ($State -eq "idle" -and (Test-Path $stateFile)) {
    $prev = try { ([System.IO.File]::ReadAllText($stateFile, [System.Text.UTF8Encoding]::new($false))).Trim() } catch { "" }
    if ($prev -eq "compacting") {
        $State = "completed"
    }
}

try {
    [System.IO.File]::WriteAllText($tempFile, $State, [System.Text.UTF8Encoding]::new($false))
    Move-Item -Force $tempFile $stateFile
} catch {
    # Best-effort fallback
    [System.IO.File]::WriteAllText($stateFile, $State, [System.Text.UTF8Encoding]::new($false))
}

exit 0
