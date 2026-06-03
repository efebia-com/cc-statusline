# Install cc-statusline (statusline + ctx-left) and wire it into Claude Code.
#
# From-source: requires Rust (https://rustup.rs) and the MSVC Build Tools (C++) for the
# bundled SQLite. Run from a clone:
#
#     .\install.ps1
#
# Honors $env:CLAUDE_CONFIG_DIR and $env:CARGO_HOME if set.
$ErrorActionPreference = 'Stop'

$ScriptDir = $PSScriptRoot
$ClaudeDir = if ($env:CLAUDE_CONFIG_DIR) { $env:CLAUDE_CONFIG_DIR } else { Join-Path $env:USERPROFILE '.claude' }
$CargoBin  = if ($env:CARGO_HOME) { Join-Path $env:CARGO_HOME 'bin' } else { Join-Path $env:USERPROFILE '.cargo\bin' }
$Statusline = Join-Path $CargoBin 'statusline.exe'

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "'cargo' not found — install Rust from https://rustup.rs"
}

Write-Host "==> Building + installing binaries (statusline, ctx-left) into $CargoBin"
cargo install --path $ScriptDir --force

Write-Host "==> Installing the ctx-left skill into $ClaudeDir\skills\ctx-left"
$SkillDir = Join-Path $ClaudeDir 'skills\ctx-left'
New-Item -ItemType Directory -Force -Path $SkillDir | Out-Null
Copy-Item (Join-Path $ScriptDir 'skill\ctx-left\SKILL.md') (Join-Path $SkillDir 'SKILL.md') -Force

Write-Host "==> Pointing statusLine.command at $Statusline (merging settings.json)"
$Settings = Join-Path $ClaudeDir 'settings.json'
if (Test-Path $Settings) {
    $data = Get-Content -Raw $Settings | ConvertFrom-Json   # preserve existing keys
} else {
    New-Item -ItemType Directory -Force -Path $ClaudeDir | Out-Null
    $data = [PSCustomObject]@{}
}
$statusLine = [PSCustomObject]@{ type = 'command'; command = $Statusline }
if ($data.PSObject.Properties.Name -contains 'statusLine') {
    $data.statusLine = $statusLine
} else {
    $data | Add-Member -NotePropertyName statusLine -NotePropertyValue $statusLine
}
$data | ConvertTo-Json -Depth 20 | Set-Content -Encoding UTF8 $Settings

Write-Host "OK. Restart Claude Code to load the statusline.   Try it:  ctx-left --all"
