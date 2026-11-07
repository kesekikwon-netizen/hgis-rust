# Helper: stage paths, refresh commit status, commit with message, optional push.
param(
  [Parameter(Mandatory = $true)][string]$Message,
  [string[]]$Path = @(),
  [switch]$Push,
  [switch]$AllTracked
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root
$env:GIT_MASTER = "1"

if ($AllTracked) {
  & git add -u
}
foreach ($p in $Path) {
  & git add -- $p
  if ($LASTEXITCODE -ne 0) { throw "git add failed: $p" }
}

& "$PSScriptRoot\update-commit-status.ps1" -IncludeStagedSummary
& git add -- docs/COMMIT_STATUS.md

& git commit -m $Message
if ($LASTEXITCODE -ne 0) { throw "git commit failed" }

if ($Push) {
  & git push origin HEAD
  if ($LASTEXITCODE -ne 0) { throw "git push failed" }
}

& git status -sb
& git log -3 --oneline
