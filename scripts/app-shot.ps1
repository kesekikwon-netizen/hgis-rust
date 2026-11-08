# Rust 앱 GUI 원격 제어 + 캡처 (지시자 검증 루프)
# 사용법: .\scripts\app-shot.ps1 -Cmd 'tab:map' -Out target\shots\map.png
param(
  [string]$Cmd = '',
  [string]$Out = 'target\shots\shot.png',
  [int]$WaitMs = 900
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$cmdPath = Join-Path $env:TEMP 'ka_hgis_debug_cmd.txt'
if ($Cmd) {
  Set-Content -Path $cmdPath -Value $Cmd -Encoding utf8 -NoNewline
  Start-Sleep -Milliseconds $WaitMs
}

if (-not ('Win32Shot' -as [type])) {
  Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win32Shot {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
}
"@
}
$p = Get-Process ka-hgis -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $p) { throw 'ka-hgis window not found' }
[void][Win32Shot]::ShowWindow($p.MainWindowHandle, 9)
[void][Win32Shot]::SetForegroundWindow($p.MainWindowHandle)
Start-Sleep -Milliseconds 350

$r = New-Object Win32Shot+RECT
[void][Win32Shot]::GetWindowRect($p.MainWindowHandle, [ref]$r)
$w = $r.R - $r.L; $h = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.L, $r.T, 0, 0, $bmp.Size)
$full = Join-Path (Get-Location) $Out
$dir = Split-Path $full -Parent
if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
$bmp.Save($full, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
"$Out (${w}x${h})"
$st = Join-Path $env:TEMP 'ka_hgis_debug_state.json'
if (Test-Path $st) { "state: " + (Get-Content $st -Raw) }
