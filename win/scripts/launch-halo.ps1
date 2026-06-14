# SessionStart helper: launch halo if not already running.
# Separated from halo-hook.ps1 to avoid the inline -Command escaping
# issues that plague plugin hook definitions.
#
# IMPORTANT: SessionStart fires on EVERY turn (including after PreCompact),
# so we must NOT kill+restart halo — that would reset its animation state
# and prevent the "compacting → completed → idle" transition from showing.
$ErrorActionPreference = 'SilentlyContinue'

# ── Write CC PID file (every turn, so halo can use it as fallback) ──
# The hook runs as a child of the Claude Code node process, so our
# parent PID IS the CC process.  This gives halo a precise target
# for process-liveness checks and focus-verification PID matching.
try {
    $ccPid = (Get-CimInstance -ClassName Win32_Process -Filter "ProcessId=$PID" -ErrorAction Stop).ParentProcessId
    if ($ccPid) {
        $pidFile = Join-Path $env:TEMP 'claude-halo-cc-pid.txt'
        [System.IO.File]::WriteAllText($pidFile, [string]$ccPid, [System.Text.UTF8Encoding]::new($false))
    }
} catch {
    # Fallback: Get-WmiObject for older PowerShell / .NET Framework
    try {
        $ccPid = (Get-WmiObject -Class Win32_Process -Filter "ProcessId=$PID").ParentProcessId
        if ($ccPid) {
            $pidFile = Join-Path $env:TEMP 'claude-halo-cc-pid.txt'
            [System.IO.File]::WriteAllText($pidFile, [string]$ccPid, [System.Text.UTF8Encoding]::new($false))
        }
    } catch {}
}

$existing = Get-Process claude-halo -ErrorAction SilentlyContinue
if (-not $existing) {
    $haloExe = Join-Path (Split-Path $PSScriptRoot -Parent) 'bin\claude-halo.exe'
    if (Test-Path $haloExe) {
        Start-Process $haloExe
    }
}
exit 0
