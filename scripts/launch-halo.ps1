# SessionStart helper: kill stale halo, then launch new one.
# Separated from halo-hook.ps1 to avoid the inline -Command escaping
# issues that plague plugin hook definitions.
$ErrorActionPreference = 'SilentlyContinue'

# 1. Kill any previous halo.exe
Get-Process claude-halo | Stop-Process -Force

# 2. Launch the real halo
$haloExe = Join-Path (Split-Path $PSScriptRoot -Parent) 'bin\claude-halo.exe'
if (Test-Path $haloExe) {
    Start-Process $haloExe
}
exit 0
