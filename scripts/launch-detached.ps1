# Launch either app detached via WMI so the agent shell's job object does not kill it.
param(
  [ValidateSet('cpp','rust')]
  [string]$Which = 'cpp'
)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$cppRef = Join-Path $root 'target\cpp-ref'

if ($Which -eq 'cpp') {
  $exe = Join-Path $root 'build\Release\ka-hgis.exe'
  $work = Split-Path $exe
  $envSetup = "`$env:PATH='C:\OSGeo4W\apps\Qt6\bin;C:\OSGeo4W\apps\qgis-dev\bin;C:\OSGeo4W\apps\gdal-dev\bin;C:\OSGeo4W\bin;'+`$env:PATH;"
} else {
  $exe = Join-Path $root 'target\debug\ka-hgis.exe'
  $work = Split-Path $exe
  $envSetup = ''
}
if (-not (Test-Path $exe)) { throw "exe not found: $exe" }

$ps = Join-Path $PSHOME 'powershell.exe'
$inner = 'cmd.exe /c "' + $ps + '" -NoProfile -ExecutionPolicy Bypass -Command "' +
         $envSetup.Replace('"','\"') + 'Start-Process -FilePath ''' + $exe + ''' -WorkingDirectory ''' + $work + '''"'
$startup = ([wmiclass]"Win32_ProcessStartup").CreateInstance()
$startup.ShowWindow = 0
$r = ([wmiclass]"Win32_Process").Create($inner, $work, $startup)
if ($r.ReturnValue -ne 0) { throw "WMI create failed: $($r.ReturnValue)" }
"detached $Which (child pid $($r.ProcessId))"