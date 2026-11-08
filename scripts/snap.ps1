param(
  [ValidateSet('cpp','rust')]
  [string]$Which = 'rust',
  [string]$Out = 'target\shots\out.png',
  [int]$Width = 1500,
  [int]$Height = 900,
  [string]$Cmd = '',
  [int]$WaitMs = 1800
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
if (-not ('SNAP' -as [type])) {
  Add-Type @"
using System;
using System.Runtime.InteropServices;
public class SNAP {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int ht, bool repaint);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();

  public static void Raise(IntPtr h) {
    if (IsIconic(h)) ShowWindow(h, 9);
    SetWindowPos(h, (IntPtr)(-1), 0, 0, 0, 0, 0x0001 | 0x0002);
    SetWindowPos(h, (IntPtr)(-2), 0, 0, 0, 0, 0x0001 | 0x0002);
    BringWindowToTop(h);
    SetForegroundWindow(h);
  }
}
"@ -ReferencedAssemblies System.Drawing
}
$want = if ($Which -eq 'cpp') { 'hgis\build\Release\ka-hgis.exe' } else { 'hgis\target\debug\ka-hgis.exe' }
$p = Get-Process ka-hgis -ErrorAction SilentlyContinue |
     Where-Object { $_.MainWindowHandle -ne 0 -and $_.Path -like "*$want" } | Select-Object -First 1
if (-not $p) { throw "$Which app not running" }
$h = $p.MainWindowHandle

[SNAP]::Raise($h)
Start-Sleep -Milliseconds 400
[void][SNAP]::MoveWindow($h, 0, 0, $Width, $Height, $true)
[SNAP]::Raise($h)
Start-Sleep -Milliseconds 1000

if ($Cmd) {
  Set-Content -Path (Join-Path $env:TEMP 'ka_hgis_debug_cmd.txt') -Value $Cmd -Encoding utf8 -NoNewline
  [SNAP]::Raise($h)
  Start-Sleep -Milliseconds $WaitMs
  [SNAP]::Raise($h)
  Start-Sleep -Milliseconds 800
}

$full = Join-Path (Split-Path $PSScriptRoot -Parent) $Out
$dir = Split-Path $full -Parent
if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }

$r = New-Object SNAP+RECT
[void][SNAP]::GetWindowRect($h, [ref]$r)
$w = $r.R - $r.L; $hh = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $hh
$g = [System.Drawing.Graphics]::FromImage($bmp)
$mode = 'screen'
if ($Which -eq 'cpp') {
  $hdc = $g.GetHdc(); [void][SNAP]::PrintWindow($h, $hdc, 2); $g.ReleaseHdc($hdc); $mode = 'print'
} else {
  $g.CopyFromScreen($r.L, $r.T, 0, 0, $bmp.Size)
}
$g.Dispose()
$bmp.Save($full, [System.Drawing.Imaging.ImageFormat]::Png)
$distinct = @{}
for ($x=60; $x -lt $bmp.Width; $x+=50) { for ($y=60; $y -lt $bmp.Height; $y+=50) { $distinct[$bmp.GetPixel($x,$y).ToArgb()]=1 } }
$bmp.Dispose()
"$Out $mode ${w}x${hh} colors=$($distinct.Count)"