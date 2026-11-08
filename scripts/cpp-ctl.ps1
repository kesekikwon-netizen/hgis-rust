param(
  [int]$AppPid = 0,
  [string]$Button = '',
  [string]$Tab = '',
  [string]$Out = '',
  [int]$WaitMs = 6000,
  [int]$Width = 1600,
  [int]$Height = 1000
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes,System.Drawing
if (-not ('W32k' -as [type])) {
  Add-Type @"
using System;
using System.Runtime.InteropServices;
public class W32k {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int ht, bool repaint);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
}
"@
}
$proc = [System.Diagnostics.Process]::GetProcessById($AppPid)
$h = $proc.MainWindowHandle
[void][W32k]::MoveWindow($h, 0, 0, $Width, $Height, $true)
Start-Sleep -Milliseconds 900
$sh = New-Object -ComObject WScript.Shell
[void]$sh.AppActivate($AppPid)
Start-Sleep -Milliseconds 800

$root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
$done = @()

if ($Button) {
  $cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Button)
  $btns = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond)
  $hit = $null
  for ($i=0; $i -lt $btns.Count; $i++) { if ($btns.Item($i).Current.Name -eq $Button) { $hit = $btns.Item($i); break } }
  if (-not $hit) { throw "button not found: $Button" }
  $inv = $hit.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
  $inv.Invoke()
  $done += "invoked button [$Button]"
  Start-Sleep -Milliseconds $WaitMs
}

if ($Tab) {
  $root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
  $tc = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::TabItem)
  $tabs = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $tc)
  $hit = $null
  for ($i=0; $i -lt $tabs.Count; $i++) { if ($tabs.Item($i).Current.Name -eq $Tab) { $hit = $tabs.Item($i); break } }
  if (-not $hit) { throw "tab not found: $Tab (have: $((0..($tabs.Count-1) | ForEach-Object { $tabs.Item($_).Current.Name }) -join ','))" }
  $sel = $hit.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern)
  $sel.Select()
  $done += "selected tab [$Tab]"
  Start-Sleep -Milliseconds $WaitMs
}

if ($Out) {
  [void]$sh.AppActivate($AppPid)
  Start-Sleep -Milliseconds 400
  [void][W32k]::MoveWindow($h, 0, 0, $Width, $Height, $true)
  Start-Sleep -Milliseconds 700
  $r = New-Object W32k+RECT
  [void][W32k]::GetWindowRect($h, [ref]$r)
  $w = $r.R - $r.L; $hh = $r.B - $r.T
  $bmp = New-Object System.Drawing.Bitmap $w, $hh
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($r.L, $r.T, 0, 0, $bmp.Size); $g.Dispose()
  $full = Join-Path (Split-Path $PSScriptRoot -Parent) $Out
  $dir = Split-Path $full -Parent
  if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
  $bmp.Save($full, [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
  $done += "$Out (${w}x${hh})"
}
$done -join ' | '