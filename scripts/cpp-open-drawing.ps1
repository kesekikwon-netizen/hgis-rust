# C++ 도면 스튜디오 열기: 리본 '도면' → '용지 설정' 모달 → '용지 만들기'
param([int]$AppPid = 0, [int]$WaitMs = 9000)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes

Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public class WEN {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern int GetWindowTextLength(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  public static System.Collections.Generic.List<string> Titles(int pid, IntPtr skip) {
    var o = new System.Collections.Generic.List<string>();
    EnumWindows((h, l) => {
      uint p2; GetWindowThreadProcessId(h, out p2);
      if (p2 == pid && h != skip && IsWindowVisible(h)) {
        int n = GetWindowTextLength(h);
        if (n > 0) { var sb = new StringBuilder(n + 2); GetWindowText(h, sb, sb.Capacity); o.Add(h.ToInt64() + "|" + sb.ToString()); }
      }
      return true;
    }, IntPtr.Zero);
    return o;
  }
}
"@
$main = (Get-Process -Id $AppPid).MainWindowHandle

function Find-Btn($hwnd, $name) {
  $root = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
  if (-not $root) { return $null }
  $c = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Button)
  $all = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $c)
  for ($i=0; $i -lt $all.Count; $i++) { if ($all.Item($i).Current.Name -eq $name) { return $all.Item($i) } }
  return $null
}

# 1) 리본 '도면' (모달이 뜨므로 Invoke 는 타임아웃날 수 있음 - 무시)
$b = Find-Btn $main '도면'
if (-not $b) { throw '도면 버튼 없음' }
try { ($b.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)).Invoke() } catch { }
Start-Sleep -Milliseconds 4000

# 2) '용지 설정' 모달 찾기
$dlgHwnd = $null
foreach ($t in [WEN]::Titles($AppPid, $main)) {
  $parts = $t.Split('|', 2)
  if ($parts[1] -match '용지') { $dlgHwnd = [IntPtr][int64]$parts[0]; "dialog: $($parts[1])" }
}
if ($dlgHwnd) {
  $mk = Find-Btn $dlgHwnd '용지 만들기'
  if ($mk) { try { ($mk.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)).Invoke(); "invoked 용지 만들기" } catch { "make-fail" } }
  else { "용지 만들기 버튼 없음" }
  Start-Sleep -Milliseconds $WaitMs
} else { "no paper dialog" }
"alive=$([bool](Get-Process -Id $AppPid -ErrorAction SilentlyContinue))"