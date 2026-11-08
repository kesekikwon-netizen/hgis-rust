# C++ 참조 앱(ka-hgis.exe) 실행 — PATH 를 스크립트 안에 고정해서 WMI 분리 실행하기.
# (WMI 로 만든 자식은 부모의 환경변수를 물려받지 않으므로 경로를 직접 박아 넣는다.)
$ErrorActionPreference = 'Stop'
$root     = Split-Path $PSScriptRoot -Parent
$devEnv   = Join-Path $root 'target\cpp-ref\scripts\dev-env.ps1'
$exe      = Join-Path $root 'build\Release\ka-hgis.exe'
$work     = Split-Path $exe
$log      = Join-Path $work 'ka-hgis-launch.log'

if (-not (Test-Path $exe))   { throw "C++ exe not found: $exe" }
if (-not (Test-Path $devEnv)) { throw "dev-env.ps1 not found: $devEnv" }

function Write-LaunchLog([string]$m) {
  $line = "[{0}] {1}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss"), $m
  Add-Content -LiteralPath $log -Value $line -Encoding UTF8
}

if ($env:KA_HGIS_CPP_DETACHED -ne '1') {
  $ps = Join-Path $PSHOME 'powershell.exe'
  if (-not (Test-Path $ps)) { $ps = 'powershell.exe' }
  $cmd = 'cmd.exe /c set KA_HGIS_CPP_DETACHED=1&& "' + $ps + '" -NoProfile -ExecutionPolicy Bypass -File "' + $PSCommandPath + '"'
  $startup = ([wmiclass]"Win32_ProcessStartup").CreateInstance()
  $startup.ShowWindow = 0
  $r = ([wmiclass]"Win32_Process").Create($cmd, $root, $startup)
  if ($r.ReturnValue -ne 0) { throw "WMI detach failed: $($r.ReturnValue)" }
  "detached via WMI child=$($r.ProcessId)"
  exit 0
}

# 여기서부터는 WMI 자식. dev-env 를 직접 불러 PATH/QGIS_PREFIX_PATH/PROJ_DATA 를 채운다.
. $devEnv
Write-LaunchLog "start exe=$exe cwd=$work OSGEO=$env:OSGEO4W_ROOT"
if (-not ($env:PATH -split ';' | Where-Object { $_ -like '*pdal-dev*' })) {
  $env:PATH = 'C:\OSGeo4W\apps\pdal-dev\bin;' + $env:PATH
}
Start-Process -FilePath $exe -WorkingDirectory $work
Write-LaunchLog "started"