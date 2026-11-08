param(
  [ValidateSet('cpp','rust')]
  [string]$Which = 'cpp',
  [string]$Out = 'target\shots\shot.png'
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
if (-not ('W32' -as [type])) {
  Add-Type @"
using System;
using System.Runtime.InteropServices;
public class W32 {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool f);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
}
"@
}
$want = if ($Which -eq 'cpp') { 'hgis\build\Release\ka-hgis.exe' } else { 'hgis\target\debug\ka-hgis.exe' }
$p = Get-Process ka-hgis -ErrorAction SilentlyContinue |
     Where-Object { $_.MainWindowHandle -ne 0 -and $_.Path -like "*$want" } | Select-Object -First 1
if (-not $p) { throw "$Which app window not found" }
$h = $p.MainWindowHandle

# Force foreground: attach to current foreground thread, then SetForegroundWindow.
$fg = [W32]::GetForegroundWindow()
$ftid = [W32]::GetWindowThreadProcessId($fg, [ref]([uint32]0))
$ctid = [W32]::GetCurrentThreadId()
[void][W32]::AttachThreadInput($ctid, $ftid, $true)
[void][W32]::ShowWindow($h, 3)
[void][W32]::BringWindowToTop($h)
[void][W32]::SetForegroundWindow($h)
[void][W32]::AttachThreadInput($ctid, $ftid, $false)
Start-Sleep -Milliseconds 1200

$r = New-Object W32+RECT
[void][W32]::GetWindowRect($h, [ref]$r)
$w = $r.R - $r.L; $h2 = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $h2
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc()
$ok = [W32]::PrintWindow($h, $hdc, 2)
$g.ReleaseHdc($hdc)
$g.Dispose()
$full = Join-Path (Split-Path $PSScriptRoot -Parent) $Out
$dir = Split-Path $full -Parent
if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
$bmp.Save($full, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
$fgAfter = [W32]::GetForegroundWindow() -eq $h
"$Out (${w}x${h2}) printwindow=$ok foreground=$fgAfter"