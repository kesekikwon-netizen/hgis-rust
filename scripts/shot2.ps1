param(
  [ValidateSet('cpp','rust')]
  [string]$Which = 'rust',
  [string]$Out = '',
  [int]$Width = 1600,
  [int]$Height = 1000,
  [string]$Cmd = '',
  [int]$WaitMs = 1600
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
if (-not ('APPS2' -as [type])) {
  Add-Type @"
using System;
using System.Runtime.InteropServices;
public class APPS2 {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int ht, bool repaint);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool f);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool RedrawWindow(IntPtr h, IntPtr r, IntPtr reg, uint flags);

  public static void Raise(IntPtr h) {
    if (IsIconic(h)) ShowWindow(h, 9);
    SetWindowPos(h, (IntPtr)(-1), 0, 0, 0, 0, 0x0001 | 0x0002);
    SetWindowPos(h, (IntPtr)(-2), 0, 0, 0, 0, 0x0001 | 0x0002);
    uint fg = 0;
    IntPtr cur = GetForegroundWindow();
    uint ftid = GetWindowThreadProcessId(cur, out fg);
    uint ctid = GetCurrentThreadId();
    AttachThreadInput(ctid, ftid, true);
    BringWindowToTop(h);
    SetForegroundWindow(h);
    AttachThreadInput(ctid, ftid, false);
    RedrawWindow(h, IntPtr.Zero, IntPtr.Zero, 0x0080 | 0x0100 | 0x0001);
  }

  public static string ShotWindow(IntPtr h, string path, bool usePrint) {
    RECT r; GetWindowRect(h, out r);
    int w = r.R - r.L, hh = r.B - r.T;
    if (w <= 0 || hh <= 0) return "bad rect";
    using (var bmp = new System.Drawing.Bitmap(w, hh))
    using (var g = System.Drawing.Graphics.FromImage(bmp)) {
      if (usePrint) {
        IntPtr hdc = g.GetHdc();
        PrintWindow(h, hdc, 2);
        g.ReleaseHdc(hdc);
      } else {
        g.CopyFromScreen(r.L, r.T, 0, 0, new System.Drawing.Size(w, hh));
      }
      bmp.Save(path, System.Drawing.Imaging.ImageFormat.Png);
      return (usePrint ? "print " : "screen ") + w + "x" + hh;
    }
  }
}
"@ -ReferencedAssemblies System.Drawing
}
$want = if ($Which -eq 'cpp') { 'hgis\build\Release\ka-hgis.exe' } else { 'hgis\target\debug\ka-hgis.exe' }
$p = Get-Process ka-hgis -ErrorAction SilentlyContinue |
     Where-Object { $_.MainWindowHandle -ne 0 -and $_.Path -like "*$want" } | Select-Object -First 1
if (-not $p) { throw "$Which app not running" }
$h = $p.MainWindowHandle

[APPS2]::Raise($h)
Start-Sleep -Milliseconds 500
[void][APPS2]::MoveWindow($h, 0, 0, $Width, $Height, $true)
[APPS2]::Raise($h)
Start-Sleep -Milliseconds 900

if ($Cmd) {
  Set-Content -Path (Join-Path $env:TEMP 'ka_hgis_debug_cmd.txt') -Value $Cmd -Encoding utf8 -NoNewline
  [APPS2]::Raise($h)
  Start-Sleep -Milliseconds $WaitMs
  [APPS2]::Raise($h)
  Start-Sleep -Milliseconds 600
}

$full = Join-Path (Split-Path $PSScriptRoot -Parent) $Out
$dir = Split-Path $full -Parent
if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
$usePrint = ($Which -eq 'cpp')
"$Out " + [APPS2]::ShotWindow($h, $full, $usePrint)