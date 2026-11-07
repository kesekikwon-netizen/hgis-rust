$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location -LiteralPath $root
. (Join-Path $root "scripts\dev-env.ps1")
$exe = Join-Path $root "build\Release\ka_workflow_tests.exe"
if (-not (Test-Path -LiteralPath $exe)) { throw "ka_workflow_tests.exe missing" }
$out = Join-Path $root "build\jeju-geo-qtest.txt"
& $exe geologyLithoWfs_excludesJejuUsesOfficialRasterUri -o $out,txt
if ($LASTEXITCODE -ne 0) { throw "geology Jeju tests failed: $LASTEXITCODE" }
Write-Host "jeju geology verification passed"
exit 0
