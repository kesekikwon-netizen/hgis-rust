param(
  [ValidateSet('cpp','rust')]
  [string]$Which = 'cpp',
  [string]$Out = '',
  [string]$Click = '',
  [int]$WaitMs = 1800
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
if (-not ('W32i' -as [type])) {
  Add-Type @"
using System;
using System.Runtime.InteropServices;
public class W32i {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int ht, bool repaint);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
}
"@
}
$want = if ($Which -eq 'cpp') { 'hgis\build\Release\ka-hgis.exe' } else { 'hgis\target\debug\ka-hgis.exe' }
$p = Get-Process ka-hgis -ErrorAction SilentlyContinue |
     Where-Object { $_.MainWindowHandle -ne 0 -and $_.Path -like "*$want" } | Select-Object -First 1
if (-not $p) { throw "$Which app window not found" }
$h = $p.MainWindowHandle

$sh = New-Object -ComObject WScript.Shell
[void]$sh.AppActivate($p.Id)
Start-Sleep -Milliseconds 700
[void][W32i]::MoveWindow($h, 0, 0, 1600, 940, $true)
[void][W32i]::SetWindowPos($h, [IntPtr](-1), 0, 0, 1600, 940, 0x0040)   # HWND_TOPMOST, no size/pos change
[void]$sh.AppActivate($p.Id)
Start-Sleep -Milliseconds 900
[void][W32i]::SetWindowPos($h, [IntPtr](-2), 0, 0, 0, 0, 0x0001 -bor 0x0002) # HWND_NOTOPMOST, NOSIZE|NOMOVE
Start-Sleep -Milliseconds 400

if ($Click) {
  $c = $Click.Split(',')
  [void][W32i]::SetCursorPos([int]$c[0], [int]$c[1])
  Start-Sleep -Milliseconds 250
  [W32i]::mouse_event(0x0002, 0, 0, 0, [IntPtr]::Zero)
  Start-Sleep -Milliseconds 120
  [W32i]::mouse_event(0x0004, 0, 0, 0, [IntPtr]::Zero)
  Start-Sleep -Milliseconds $WaitMs
}

if ($Out) {
  $r = New-Object W32i+RECT
  [void][W32i]::GetWindowRect($h, [ref]$r)
  $w = $r.R - $r.L; $hh = $r.B - $r.T
  $bmp = New-Object System.Drawing.Bitmap $w, $hh
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($r.L, $r.T, 0, 0, $bmp.Size)
  $g.Dispose()
  $full = Join-Path (Split-Path $PSScriptRoot -Parent) $Out
  $dir = Split-Path $full -Parent
  if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
  $bmp.Save($full, [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
  "$Out (${w}x${hh}) fg=$([W32i]::GetForegroundWindow() -eq $h)"
} else { "ok $Which" }