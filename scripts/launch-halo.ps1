# SessionStart helper: launch halo if not already running.
# Separated from halo-hook.ps1 to avoid the inline -Command escaping
# issues that plague plugin hook definitions.
#
# IMPORTANT: SessionStart fires on EVERY turn (including after PreCompact),
# so we must NOT kill+restart halo — that would reset its animation state
# and prevent the "compacting → completed → idle" transition from showing.
$ErrorActionPreference = 'SilentlyContinue'

$existing = Get-Process claude-halo -ErrorAction SilentlyContinue
if (-not $existing) {
    $haloExe = Join-Path (Split-Path $PSScriptRoot -Parent) 'bin\claude-halo.exe'
    if (Test-Path $haloExe) {
        Start-Process $haloExe
    }
}
exit 0
