# 두 앱이 지금 실행 가능한 상태인지 한 번에 점검한다.
# 사용법: .\scripts\check-apps.ps1
$ErrorActionPreference = 'Continue'
$root = Split-Path $PSScriptRoot -Parent
$ok = $true
$sysPath = 'C:\Windows\System32;C:\Windows;C:\Windows\System32\Wbem'

Write-Host "=== 1. Rust 앱 (OSGeo4W 불필요) ===" -ForegroundColor Cyan
$rustExe = Join-Path $root 'target\debug\ka-hgis.exe'
if (Test-Path $rustExe) {
    $env:PATH = $sysPath
    $out = & "$env:ComSpec" /c "`"$rustExe`" --smoke-quit 2>&1"
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  OK  smoke exit 0" -ForegroundColor Green
    } else {
        Write-Host "  FAIL smoke exit $LASTEXITCODE" -ForegroundColor Red
        $out | Select-Object -First 5 | ForEach-Object { Write-Host "       $_" }
        $ok = $false
    }
} else {
    Write-Host "  MISSING $rustExe" -ForegroundColor Red
    $ok = $false
}

Write-Host "=== 2. C++ 참조 앱 의존 DLL ===" -ForegroundColor Cyan
$exe = Join-Path $root 'build\Release\ka-hgis.exe'
if (-not (Test-Path $exe)) {
    Write-Host "  MISSING $exe" -ForegroundColor Red
    $ok = $false
} else {
    $env:PATH = @(
        'C:\OSGeo4W\apps\Qt6\bin',
        'C:\OSGeo4W\apps\qgis-dev\bin',
        'C:\OSGeo4W\apps\gdal-dev\bin',
        'C:\OSGeo4W\apps\pdal-dev\bin',
        'C:\OSGeo4W\bin'
    ) -join ';'
    $env:PATH = $env:PATH + ';' + $sysPath
    $env:OSGEO4W_ROOT = 'C:\OSGeo4W'
    $env:QGIS_PREFIX_PATH = 'C:\OSGeo4W\apps\qgis-dev'
    $proj = 'C:\OSGeo4W\share\proj'
    if (Test-Path $proj) { $env:PROJ_DATA = $proj }
    $env:GDAL_DATA = 'C:\OSGeo4W\apps\gdal-dev\share\gdal'

    $out = & "$env:ComSpec" /c "`"$exe`" --smoke-quit 2>&1"
    $code = $LASTEXITCODE
    if ($code -eq 0) {
        Write-Host "  OK  smoke exit 0" -ForegroundColor Green
    } else {
        Write-Host "  FAIL smoke exit $code" -ForegroundColor Red
        $out | Select-Object -First 5 | ForEach-Object { Write-Host "       $_" }
        $ok = $false
    }
}

Write-Host "=== 3. 실행 중인 앱 ===" -ForegroundColor Cyan
Get-Process ka-hgis -ErrorAction SilentlyContinue |
    Select-Object Id, MainWindowTitle, @{n = 'folder'; e = { Split-Path $_.Path -Parent }} |
    Format-Table -AutoSize | Out-String -Width 140 | Write-Host

if ($ok) { Write-Host "`n전부 정상" -ForegroundColor Green }
else { Write-Host "`n문제 있음 — 위 FAIL 을 확인" -ForegroundColor Red; exit 1 }
