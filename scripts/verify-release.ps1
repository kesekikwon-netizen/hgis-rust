param([switch]$CheckOnly)

$ErrorActionPreference = 'Stop'
$releaseRoot = Split-Path -Parent $PSScriptRoot
$receiptPath = Join-Path $releaseRoot 'build/release-verified.json'
$releaseExe = Join-Path $releaseRoot 'build/Release/ka-hgis.exe'

function Get-ReleaseFingerprint {
  $paths = @(& git -C $releaseRoot -c core.quotepath=false ls-files --cached --others --exclude-standard -- src tests data cmake scripts launch.ps1 CMakeLists.txt CMakePresets.json)
  if ($LASTEXITCODE -ne 0 -or $paths.Count -eq 0) { throw 'Cannot enumerate release inputs.' }
  $lines = foreach ($path in ($paths | Sort-Object -Unique)) {
    $absolute = Join-Path $releaseRoot $path
    if (Test-Path -LiteralPath $absolute -PathType Leaf) {
      $hash = (Get-FileHash -LiteralPath $absolute -Algorithm SHA256).Hash
      "$path $hash"
    } else {
      "$path MISSING"
    }
  }
  $exeHash = (Get-FileHash -LiteralPath $releaseExe -Algorithm SHA256).Hash
  $bytes = [Text.Encoding]::UTF8.GetBytes(($lines -join "`n") + "`nEXE $exeHash")
  $sha = [Security.Cryptography.SHA256]::Create()
  try { return ([BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-', '') }
  finally { $sha.Dispose() }
}

if ($CheckOnly) {
  if (-not (Test-Path -LiteralPath $receiptPath)) {
    throw 'No verified Release. Run scripts/verify-release.ps1 before publishing.'
  }
  $receipt = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
  if ($receipt.schema -ne 1 -or $receipt.ctestPassed -ne $true -or
      $receipt.smokePassed -ne $true -or $receipt.fingerprint -ne (Get-ReleaseFingerprint)) {
    throw 'Release inputs changed or checks did not pass. Run scripts/verify-release.ps1 again.'
  }
  Write-Host 'Verified Release matches the current source, theme, tests and executable.'
  exit 0
}

# A failed new attempt must never leave an older success receipt usable.
if (Test-Path -LiteralPath $receiptPath) { Remove-Item -LiteralPath $receiptPath }
Push-Location $releaseRoot
$oldPlatform = $env:QT_QPA_PLATFORM
try {
  $env:PATH = 'C:/Program Files/CMake/bin;' + $env:PATH
  . (Join-Path $PSScriptRoot 'dev-env.ps1')
  cmake -S . -B build -G 'Visual Studio 17 2022' -A x64 "-DOSGEO4W_ROOT=$env:OSGEO4W_ROOT" -DKA_HGIS_BUILD_TESTS=ON
  if ($LASTEXITCODE -ne 0) { throw 'Release configure failed.' }
  cmake --build build --config Release
  if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }
  $fingerprint = Get-ReleaseFingerprint
  $env:QT_QPA_PLATFORM = 'offscreen'
  ctest --test-dir build -C Release --output-on-failure --no-tests=error --output-junit release-tests.xml
  if ($LASTEXITCODE -ne 0) { throw 'Release tests failed; publishing remains blocked.' }
  & (Join-Path $PSScriptRoot 'run-ka-hgis.ps1') --smoke-quit
  if ($LASTEXITCODE -ne 0) { throw 'Release startup smoke failed; publishing remains blocked.' }
  if ($fingerprint -ne (Get-ReleaseFingerprint)) { throw 'Release inputs changed during verification.' }
  [ordered]@{
    schema = 1
    verifiedUtc = [DateTime]::UtcNow.ToString('o')
    fingerprint = $fingerprint
    executableSha256 = (Get-FileHash -LiteralPath $releaseExe -Algorithm SHA256).Hash
    ctestPassed = $true
    smokePassed = $true
  } | ConvertTo-Json | Set-Content -LiteralPath $receiptPath -Encoding UTF8
  Write-Host 'Release verified for the desktop shortcut. Portable output was not changed.'
} finally {
  $env:QT_QPA_PLATFORM = $oldPlatform
  Pop-Location
}
