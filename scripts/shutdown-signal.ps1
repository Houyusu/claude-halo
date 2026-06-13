# Creates the shutdown signal file that tells halo.exe to exit.
# Called by the Stop hook when Claude Code exits.
# This MUST be a separate script (not -Command inline) to avoid
# multi-level escaping issues with PowerShell + JSON + cmd.exe.
$signalFile = Join-Path $env:TEMP 'claude-halo-shutdown.txt'
try {
    [System.IO.File]::WriteAllText($signalFile, '1', [System.Text.UTF8Encoding]::new($false))
} catch {
    # Best-effort fallback
    '1' | Out-File -FilePath $signalFile -Encoding ascii -Force
}
exit 0
