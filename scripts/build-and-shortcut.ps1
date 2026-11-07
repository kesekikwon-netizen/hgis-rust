# Build Release, then (re)create the desktop shortcut.
# The shortcut runs wscript -> start-ka-hgis.vbs -> launch.ps1 -> build\Release\ka-hgis.exe,
# so it always starts the latest build of THIS checkout without editing it after each rebuild.
#   .\scripts\build-and-shortcut.ps1               # build + shortcut
#   .\scripts\build-and-shortcut.ps1 -ShortcutOnly # shortcut only
param([switch]$ShortcutOnly)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

if (-not $ShortcutOnly) {
  & (Join-Path $PSScriptRoot "build-now.ps1")
}

$exe = Join-Path $root "build\Release\ka-hgis.exe"
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw "Release ka-hgis.exe missing: $exe" }
$vbs = Join-Path $PSScriptRoot "start-ka-hgis.vbs"
# Same icon file as update-app-icon.ps1 uses for the shortcut (the exe embeds it too).
$ico = Join-Path $root "data\theme\ka-hgis.ico"
$wscript = Join-Path $env:SystemRoot "System32\wscript.exe"

# The Desktop known folder may be redirected to OneDrive.
$desktop = [Environment]::GetFolderPath("Desktop")
# App name from code points: PowerShell 5.1 reads BOM-less .ps1 as ANSI.
$name = (-join [char[]](0xACE0, 0xACE0, 0xD559, 0x20, 0xC804, 0xC6A9)) + " HGIS"
$lnk = Join-Path $desktop "$name.lnk"
$s = (New-Object -ComObject WScript.Shell).CreateShortcut($lnk)
$s.TargetPath = $wscript
$s.Arguments = "//nologo `"$vbs`""
$s.WorkingDirectory = $root
$s.IconLocation = "$ico,0"
$s.Description = $name
$s.Save()
# Explorer caches icons per path; tell it this shortcut and the icon changed.
Add-Type -Namespace KaHgis -Name ShellNotify -MemberDefinition '[DllImport("shell32.dll", CharSet = CharSet.Unicode)] public static extern void SHChangeNotify(int e, uint f, string a, System.IntPtr b);'
[KaHgis.ShellNotify]::SHChangeNotify(0x00002000, 0x0005, $lnk, [IntPtr]::Zero)  # SHCNE_UPDATEITEM
[KaHgis.ShellNotify]::SHChangeNotify(0x08000000, 0x0000, $null, [IntPtr]::Zero) # SHCNE_ASSOCCHANGED
& (Join-Path $env:SystemRoot "System32\ie4uinit.exe") -show
Write-Host "SHORTCUT: $lnk -> $vbs (exe $exe)"
