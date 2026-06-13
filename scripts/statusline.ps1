[Console]::InputEncoding = [System.Text.Encoding]::UTF8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8
$PSDefaultParameterValues['*:Encoding'] = 'utf8'
$in = [Console]::In.ReadToEnd()
if (-not $in) { exit 0 }
try { $j = $in | ConvertFrom-Json } catch { exit 0 }

$sections = @()

# dir + model
$dir = Split-Path $j.workspace.current_dir -Leaf
$left = "[$dir] $($j.model.display_name)"
$sections += $left

# context (percentage only) with color
$rp = $j.context_window.remaining_percentage
if ($null -ne $rp) {
    if ($rp -le 30) {
        $color = "$([char]27)[31m"    # red
    } elseif ($rp -le 60) {
        $color = "$([char]27)[33m"    # yellow
    } else {
        $color = ""                   # default
    }
    $reset = if ($color) { "$([char]27)[0m" } else { "" }
    $sections += "${color}ctx ${rp}%${reset}"
}

# status
$st = @()
if ($j.exceeds_200k_tokens) { $st += "!200K" }
if ($j.vim -and $j.vim.mode) { $st += $j.vim.mode }
if ($j.output_style -and $j.output_style.name -ne 'default') { $st += $j.output_style.name }
if ($j.agent -and $j.agent.name) { $st += "agent $($j.agent.name)" }
if ($j.worktree -and $j.worktree.name) { $st += "wt $($j.worktree.name)" }
if ($j.workspace.git_worktree) { $st += $j.workspace.git_worktree }
if ($st.Count -gt 0) { $sections += ($st -join ' ') }

# duration
$d = [math]::Floor($j.cost.total_duration_ms / 60000)
if ($d -gt 0) { $sections += "session ${d}m" }

# rate limits
$rl = $j.rate_limits
if ($rl) {
    $rp = @()
    if ($rl.five_hour) { $rp += "5h $([math]::Round($rl.five_hour.used_percentage))%" }
    if ($rl.seven_day) { $rp += "7d $([math]::Round($rl.seven_day.used_percentage))%" }
    if ($rp.Count -gt 0) { $sections += ($rp -join ' ') }
}

Write-Output ($sections -join ' | ')
