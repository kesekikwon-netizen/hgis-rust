# C++ 앱: 홈→조사 열기→도면 탭까지 자동 진행 후 캡처
param(
  [int]$AppPid = 0,
  [string]$OpenSurvey = '',
  [string]$Tab = '',
  [string]$Out = 'target\shots\cpp.png',
  [int]$Width = 2200,
  [int]$Height = 940
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes,System.Drawing
if (-not ('CDRV' -as [type])) {
  Add-Type @"
using System;
using System.Runtime.InteropServices;
public class CDRV {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int ht, bool repaint);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
}
"@ -ReferencedAssemblies System.Drawing
}
$proc = [System.Diagnostics.Process]::GetProcessById($AppPid)
$h = $proc.MainWindowHandle
[void][CDRV]::MoveWindow($h, 0, 0, $Width, $Height, $true)
Start-Sleep -Milliseconds 1200
$sh = New-Object -ComObject WScript.Shell
[void]$sh.AppActivate($AppPid); Start-Sleep -Milliseconds 900

function Get-Root { [System.Windows.Automation.AutomationElement]::FromHandle($h) }
function Find-Item($root, $type, $name) {
  $c = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, $type)
  $all = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $c)
  for ($i=0; $i -lt $all.Count; $i++) { if ($all.Item($i).Current.Name -eq $name) { return $all.Item($i) } }
  return $null
}

$log = @()
$tabItemType = [System.Windows.Automation.ControlType]::TabItem
$buttonType = [System.Windows.Automation.ControlType]::Button

# 1) 홈 탭
$homeTab = Find-Item (Get-Root) $tabItemType '홈'
if ($homeTab) { try { ($homeTab.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern)).Select(); $log += 'home' } catch { $log += 'home-fail' } }
Start-Sleep -Milliseconds 1500

# 2) 이어서 열기
if ($OpenSurvey) {
  $btn = Find-Item (Get-Root) $buttonType $OpenSurvey
  if ($btn) { try { ($btn.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)).Invoke(); $log += "invoked:$OpenSurvey"; Start-Sleep -Seconds 22 } catch { $log += 'invoke-fail' } }
  else { $log += "notfound:$OpenSurvey" }
}

# 3) 탭 이동
if ($Tab) {
  $t = Find-Item (Get-Root) $tabItemType $Tab
  if ($t) { try { ($t.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern)).Select(); $log += "tab:$Tab" } catch { $log += "tab-fail:$Tab" } }
  else {
    $b = Find-Item (Get-Root) $buttonType $Tab
    if ($b) { try { ($b.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)).Invoke(); $log += "btn:$Tab"; Start-Sleep -Seconds 10 } catch { $log += "btn-fail:$Tab" } }
    else { $log += "tabmiss:$Tab" }
  }
  Start-Sleep -Seconds 6
}

[void]$sh.AppActivate($AppPid); Start-Sleep -Milliseconds 800
[void][CDRV]::MoveWindow($h, 0, 0, $Width, $Height, $true); Start-Sleep -Milliseconds 900
$r = New-Object CDRV+RECT
[void][CDRV]::GetWindowRect($h, [ref]$r)
$w = $r.R - $r.L; $hh = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $hh
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc(); [void][CDRV]::PrintWindow($h, $hdc, 2); $g.ReleaseHdc($hdc); $g.Dispose()
$full = Join-Path (Split-Path $PSScriptRoot -Parent) $Out
$bmp.Save($full, [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
($log -join ' | ') + " -> $Out ${w}x${hh}"