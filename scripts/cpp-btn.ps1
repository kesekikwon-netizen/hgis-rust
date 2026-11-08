# C++ 앱: 리본 버튼을 이름으로 눌러 스튜디오를 열고 캡처
param(
  [int]$AppPid = 0,
  [string]$Button = '도면',
  [string]$Out = 'target\shots\cpp.png',
  [int]$Width = 1500,
  [int]$Height = 900,
  [int]$WaitMs = 12000
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes,System.Drawing
if (-not ('CDRV2' -as [type])) {
  Add-Type @"
using System;
using System.Runtime.InteropServices;
public class CDRV2 {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int ht, bool repaint);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
}
"@ -ReferencedAssemblies System.Drawing
}
$proc = [System.Diagnostics.Process]::GetProcessById($AppPid)
$h = $proc.MainWindowHandle
[void][CDRV2]::MoveWindow($h, 0, 0, $Width, $Height, $true)
Start-Sleep -Milliseconds 1200
$sh = New-Object -ComObject WScript.Shell
[void]$sh.AppActivate($AppPid); Start-Sleep -Milliseconds 900

$root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
$bc = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Button)
$btns = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $bc)
$log = @()
$hit = $null
for ($i=0; $i -lt $btns.Count; $i++) { if ($btns.Item($i).Current.Name -eq $Button) { $hit = $btns.Item($i); break } }
if ($hit) {
  try { ($hit.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)).Invoke(); $log += "invoked:$Button" }
  catch { $log += "invoke-timeout:$Button" }
} else { $log += "notfound:$Button" }
Start-Sleep -Milliseconds $WaitMs

# 모달이 떠 있으면 Esc 로 닫기
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public class MOD {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  public static System.Collections.Generic.List<IntPtr> Modals(int pid, IntPtr main) {
    var o = new System.Collections.Generic.List<IntPtr>();
    EnumWindows((h, l) => {
      uint p2; GetWindowThreadProcessId(h, out p2);
      if (p2 == pid && h != main && IsWindowVisible(h)) {
        int n = GetWindowTextLength(h);
        if (n > 0) o.Add(h);
      }
      return true;
    }, IntPtr.Zero);
    return o;
  }
  [DllImport("user32.dll")] public static extern int GetWindowTextLength(IntPtr h);
}
"@
$mods = [MOD]::Modals($AppPid, $h)
foreach ($m in $mods) {
  [void][MOD]::PostMessage($m, 0x0100, [IntPtr]0x1B, [IntPtr]0x00010001)
  Start-Sleep -Milliseconds 120
  [void][MOD]::PostMessage($m, 0x0101, [IntPtr]0x1B, [IntPtr]0xC0010001)
  $log += "esc-modal"
}
if ($mods.Count -gt 0) { Start-Sleep -Seconds 4 }

[void]$sh.AppActivate($AppPid); Start-Sleep -Milliseconds 800
[void][CDRV2]::MoveWindow($h, 0, 0, $Width, $Height, $true); Start-Sleep -Milliseconds 900
$r = New-Object CDRV2+RECT
[void][CDRV2]::GetWindowRect($h, [ref]$r)
$w = $r.R - $r.L; $hh = $r.B - $r.T
$bmp = New-Object System.Drawing.Bitmap $w, $hh
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc(); [void][CDRV2]::PrintWindow($h, $hdc, 2); $g.ReleaseHdc($hdc); $g.Dispose()
$full = Join-Path (Split-Path $PSScriptRoot -Parent) $Out
$bmp.Save($full, [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
($log -join ' | ') + " -> $Out ${w}x${hh}"