# DPI-aware capture for the Rust app.
# PowerShell is DPI-unaware (96 DPI) while ka-hgis.exe is Per-Monitor aware.
# Without SetProcessDPIAware, MoveWindow/GetWindowRect are scaled by 1.5x on
# this display and CopyFromScreen copies the wrong region (Director's analysis).
param(
  [string]$Tab = 'tab:home',
  [string]$Out = 'target/shots/shot.png',
  [int]$Width = 1600,
  [int]$Height = 900,
  [int]$WaitMs = 5000
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
if (-not ('DPIW' -as [type])) {
  Add-Type @"
using System;
using System.Runtime.InteropServices;
public class DPIW {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int ht, bool repaint);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
}
"@ -ReferencedAssemblies System.Drawing
}
# must happen before any window measurement
[void][DPIW]::SetProcessDPIAware()

$p = Get-Process ka-hgis -ErrorAction SilentlyContinue |
     Where-Object { $_.Path -like '*target\debug\ka-hgis.exe' } | Select-Object -First 1
if (-not $p) { throw 'target/debug/ka-hgis.exe not running' }
$h = $p.MainWindowHandle

# place the window at true physical pixels
[void][DPIW]::SetWindowPos($h, [IntPtr](-1), 0, 0, 0, 0, 0x0001 -bor 0x0002)
[void][DPIW]::SetWindowPos($h, [IntPtr](-2), 0, 0, 0, 0, 0x0001 -bor 0x0002)
[void][DPIW]::MoveWindow($h, 0, 0, $Width, $Height, $true)
[void][DPIW]::BringWindowToTop($h)
[void][DPIW]::SetForegroundWindow($h)
Start-Sleep -Milliseconds 800

if ($Tab) {
  Set-Content -Path (Join-Path $env:TEMP 'ka_hgis_debug_cmd.txt') -Value $Tab -Encoding utf8 -NoNewline
  Start-Sleep -Milliseconds $WaitMs
  [void][DPIW]::BringWindowToTop($h)
  [void][DPIW]::SetForegroundWindow($h)
  Start-Sleep -Milliseconds 900
}

$r = New-Object DPIW+RECT
[void][DPIW]::GetWindowRect($h, [ref]$r)
$c = New-Object DPIW+RECT
[void][DPIW]::GetClientRect($h, [ref]$c)
$w = $r.R - $r.L; $hh = $r.B - $r.T
if ($w -lt 100 -or $hh -lt 100) { throw "bad window rect ${w}x${hh}" }

$bmp = New-Object System.Drawing.Bitmap($w, $hh)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.L, $r.T, 0, 0, $bmp.Size)
$g.Dispose()
$full = Join-Path (Split-Path $PSScriptRoot -Parent) $Out
$dir = Split-Path $full -Parent
if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
$bmp.Save($full, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
"$Out  window=${w}x${hh}  client=$($c.R-$c.L)x$($c.B-$c.T)  dpi=$([DPIW]::GetDpiForWindow($h))"
