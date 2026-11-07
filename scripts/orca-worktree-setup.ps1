# Orca worktree setup for ka-hgis (Windows).
# Orca runs this from the NEW worktree root after git worktree add.
# Does not compile the app (field check stays the desktop icon).
# Does not share build/ (CMake stores absolute source paths).
# Does not copy .unlazy/ leftover gates.

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
if (-not (Test-Path -LiteralPath (Join-Path $Root "AGENTS.md"))) {
  $Root = (Get-Location).Path
}
Set-Location -LiteralPath $Root

function Write-Step([string]$msg) { Write-Host "==> $msg" }

Write-Step "ka-hgis Orca worktree setup"
Write-Host "KA_HGIS_ROOT=$Root"

if (-not (Test-Path -LiteralPath (Join-Path $Root "AGENTS.md"))) {
  throw "Not a ka-hgis tree (AGENTS.md missing)."
}
if (-not (Test-Path -LiteralPath (Join-Path $Root "src\core\LayerOps.h"))) {
  throw "Not a ka-hgis tree (src/core/LayerOps.h missing)."
}

$cmakeCandidates = @(
  "C:\CMake\bin",
  (Join-Path ${env:ProgramFiles} "CMake\bin"),
  (Join-Path ${env:ProgramFiles(x86)} "CMake\bin")
)
foreach ($c in $cmakeCandidates) {
  if (Test-Path -LiteralPath (Join-Path $c "cmake.exe")) {
    $env:PATH = "$c;" + $env:PATH
    break
  }
}
if (-not (Get-Command cmake -ErrorAction SilentlyContinue)) {
  throw "CMake not on PATH. Install to C:\CMake\bin."
}

$devEnv = Join-Path $Root "scripts\dev-env.ps1"
if (-not (Test-Path -LiteralPath $devEnv)) {
  throw "scripts\dev-env.ps1 missing."
}
. $devEnv
if (-not $env:OSGEO4W_ROOT -or -not (Test-Path -LiteralPath $env:OSGEO4W_ROOT)) {
  throw "OSGEO4W_ROOT missing after dev-env.ps1."
}
if (-not $env:QGIS_PREFIX_PATH) {
  throw "QGIS_PREFIX_PATH missing after dev-env.ps1."
}

$node = Get-Command node -ErrorAction SilentlyContinue
if (-not $node) {
  Write-Host "WARN node not on PATH — /unlazy gate-check needs Node >= 16"
} else {
  Write-Host ("node " + (& node --version))
}

if (Test-Path -LiteralPath (Join-Path $Root ".unlazy")) {
  Write-Host "NOTE leftover .unlazy/ in this tree — scoped GATES can arm Stop; wipe after the task."
}

Write-Host "NOTE live tiles use VworldSettings / 도움말. Never commit a production key."

$buildDir = Join-Path $Root "build"
$cache = Join-Path $buildDir "CMakeCache.txt"
if (-not (Test-Path -LiteralPath $cache)) {
  Write-Step "cmake configure (worktree has no build cache)"
  cmake -S $Root -B $buildDir -G "Visual Studio 17 2022" -A x64 `
    "-DOSGEO4W_ROOT=$env:OSGEO4W_ROOT" `
    "-DKA_HGIS_BUILD_TESTS=ON"
  if ($LASTEXITCODE -ne 0) { throw "cmake configure failed: $LASTEXITCODE" }
} else {
  Write-Host "cmake cache already present — skip configure"
}

Write-Host "invariants: Architecture B; export EPSG:5179; no QGIS fork; no DXF submit"
Write-Host "KA_HGIS_WORKTREE_SETUP_OK"
exit 0
