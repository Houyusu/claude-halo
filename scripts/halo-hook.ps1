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

exit 0
