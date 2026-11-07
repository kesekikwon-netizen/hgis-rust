# Point this repo at .githooks so commit status auto-updates.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

if (-not (Test-Path ".git")) { throw "Run inside git repo root" }
if (-not (Test-Path ".githooks\pre-commit")) { throw "Missing .githooks/pre-commit" }

& git config core.hooksPath .githooks
$hooksPath = & git config --get core.hooksPath
Write-Host "core.hooksPath = $hooksPath"
Write-Host "Git hooks installed. Every commit will refresh docs/COMMIT_STATUS.md"
