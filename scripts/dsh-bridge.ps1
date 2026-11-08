# Antigravity -> DeepSeek Harness 제어 및 C++/Rust 실화면 대조 브리지
# 사용법:
#   powershell -ExecutionPolicy Bypass -File scripts\dsh-bridge.ps1 -Action status
#   powershell -ExecutionPolicy Bypass -File scripts\dsh-bridge.ps1 -Action send -Text "DEEPSEEK_WORKER_INSTRUCTIONS.md 의 Task D1 을 수행하라."
#   powershell -ExecutionPolicy Bypass -File scripts\dsh-bridge.ps1 -Action compare -Cmd "tab:home"
param(
  [ValidateSet('status','send','submit','stop-turn','compare')]
  [string]$Action = 'status',
  [string]$Text = '',
  [string]$Cmd = 'tab:home',
  [int]$Width = 1600,
  [int]$Height = 900
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

if (-not ('DSHBridge' -as [type])) {
  Add-Type @"
using System;
using System.Runtime.InteropServices;
public class DSHBridge {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint idAttach, uint idAttachTo, bool fAttach);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint dwFlags, int dx, int dy, uint dwData, UIntPtr dwExtraInfo);
  public static void ForceForeground(IntPtr h) {
    IntPtr fg = GetForegroundWindow();
    uint dummy = 0;
    uint fgThread = GetWindowThreadProcessId(fg, out dummy);
    uint curThread = GetCurrentThreadId();
    if (fgThread != curThread) AttachThreadInput(curThread, fgThread, true);
    ShowWindow(h, 9);
    BringWindowToTop(h);
    SetForegroundWindow(h);
    if (fgThread != curThread) AttachThreadInput(curThread, fgThread, false);
  }
  public static void ClickAt(int x, int y) {
    SetCursorPos(x, y);
    System.Threading.Thread.Sleep(60);
    mouse_event(0x0002, 0, 0, 0, UIntPtr.Zero);
    System.Threading.Thread.Sleep(40);
    mouse_event(0x0004, 0, 0, 0, UIntPtr.Zero);
  }
}
"@ -ReferencedAssemblies System.Drawing
}
[void][DSHBridge]::SetProcessDPIAware()

function Get-DshWindow {
  $p = Get-Process -Name "DeepSeek Harness" -ErrorAction SilentlyContinue |
       Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
  if (-not $p) { throw "DeepSeek Harness window not found" }
  return $p
}

function Save-WindowShot([IntPtr]$h, [string]$outPath) {
  $r = New-Object DSHBridge+RECT
  [void][DSHBridge]::GetWindowRect($h, [ref]$r)
  if ([DSHBridge]::IsIconic($h) -or ($r.R - $r.L) -lt 600) {
    [void][DSHBridge]::ShowWindow($h, 9)
    Start-Sleep -Milliseconds 300
    [void][DSHBridge]::GetWindowRect($h, [ref]$r)
  }
  $w = [Math]::Max(320, $r.R - $r.L)
  $hh = [Math]::Max(240, $r.B - $r.T)
  $bmp = New-Object System.Drawing.Bitmap $w, $hh
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $hdc = $g.GetHdc()
  [void][DSHBridge]::PrintWindow($h, $hdc, 2)
  $g.ReleaseHdc($hdc)
  $g.Dispose()
  $bmp.Save($outPath, [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
  return @{ L = $r.L; T = $r.T; R = $r.R; B = $r.B; W = $w; H = $hh }
}

switch ($Action) {
  'status' {
    $p = Get-DshWindow
    $info = Save-WindowShot -h $p.MainWindowHandle -outPath "target\shots\dsh-window-full.png"
    $sub = Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -like "*dsh-subprocess-local*" }
    "DSH_PID=$($p.Id) RECT=$($info.W)x$($info.H) SUBPROCESS_COUNT=$(@($sub).Count)"
  }
  'stop-turn' {
    # Stop any sleeping powershell child of DeepSeek Harness and click the Stop button if running
    Get-CimInstance Win32_Process | Where-Object {
      $_.Name -eq 'powershell.exe' -and $_.CommandLine -like "*Start-Sleep*"
    } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
    $p = Get-DshWindow
    $h = $p.MainWindowHandle
    if ([DSHBridge]::IsIconic($h)) { [void][DSHBridge]::ShowWindow($h, 9) }
    [void][DSHBridge]::BringWindowToTop($h)
    [void][DSHBridge]::SetForegroundWindow($h)
    Start-Sleep -Milliseconds 250
    $r = New-Object DSHBridge+RECT
    [void][DSHBridge]::GetWindowRect($h, [ref]$r)
    [DSHBridge]::ClickAt($r.R - 231, $r.B - 92)
    Start-Sleep -Milliseconds 500
    $info = Save-WindowShot -h $h -outPath "target\shots\dsh-window-full.png"
    "STOPPED_TURN RECT=$($info.W)x$($info.H)"
  }
  'send' {
    if (-not $Text) { throw "-Text is required for send action" }
    $p = Get-DshWindow
    $h = $p.MainWindowHandle
    $r = New-Object DSHBridge+RECT
    [void][DSHBridge]::GetWindowRect($h, [ref]$r)
    if ([DSHBridge]::IsIconic($h) -or ($r.R - $r.L) -lt 800) {
      [void][DSHBridge]::ShowWindow($h, 9)
      Start-Sleep -Milliseconds 350
    }
    [DSHBridge]::ForceForeground($h)
    Start-Sleep -Milliseconds 350
    [void][DSHBridge]::GetWindowRect($h, [ref]$r)
    $w = $r.R - $r.L
    $hh = $r.B - $r.T
    # Input box center inside DeepSeek Harness window (never click top-left sidebar toggle!)
    $clickX = $r.L + [int]($w * 0.45)
    $clickY = $r.B - 135
    Set-Clipboard -Value $Text
    [DSHBridge]::ClickAt($clickX, $clickY)
    Start-Sleep -Milliseconds 250
    [DSHBridge]::ClickAt($clickX, $clickY)
    Start-Sleep -Milliseconds 200
    [System.Windows.Forms.SendKeys]::SendWait("^a")
    Start-Sleep -Milliseconds 100
    [System.Windows.Forms.SendKeys]::SendWait("^v")
    Start-Sleep -Milliseconds 350
    [System.Windows.Forms.SendKeys]::SendWait("{ENTER}")
    Start-Sleep -Milliseconds 1100
    $info = Save-WindowShot -h $h -outPath "target\shots\dsh-window-full.png"
    "SENT_TO_DSH CLICK=($clickX,$clickY) RECT=($($r.L),$($r.T),$($r.R),$($r.B)) SIZE=$($info.W)x$($info.H)"
  }
  'submit' {
    $p = Get-DshWindow
    $h = $p.MainWindowHandle
    [DSHBridge]::ForceForeground($h)
    Start-Sleep -Milliseconds 250
    $r = New-Object DSHBridge+RECT
    [void][DSHBridge]::GetWindowRect($h, [ref]$r)
    [DSHBridge]::ClickAt($r.R - 80, $r.B - 80)
    Start-Sleep -Milliseconds 900
    $info = Save-WindowShot -h $h -outPath "target\shots\dsh-window-full.png"
    "SUBMITTED_DSH RECT=($($r.L),$($r.T),$($r.R),$($r.B)) SIZE=$($info.W)x$($info.H)"
  }
  'compare' {
    $cpp = Get-Process ka-hgis -ErrorAction SilentlyContinue |
           Where-Object { $_.MainWindowHandle -ne 0 -and $_.Path -like "*build\Release\ka-hgis.exe" } | Select-Object -First 1
    $rst = Get-Process ka-hgis -ErrorAction SilentlyContinue |
           Where-Object { $_.MainWindowHandle -ne 0 -and $_.Path -like "*target\debug\ka-hgis.exe" } | Select-Object -First 1
    if ($cpp) {
      $cInfo = Save-WindowShot -h $cpp.MainWindowHandle -outPath "target\shots\cmp-cpp.png"
      "CPP_SHOT=$($cInfo.W)x$($cInfo.H)"
    }
    if ($rst) {
      if ($Cmd) {
        Set-Content -Path (Join-Path $env:TEMP 'ka_hgis_debug_cmd.txt') -Value $Cmd -Encoding utf8 -NoNewline
        Start-Sleep -Milliseconds 1200
      }
      $rInfo = Save-WindowShot -h $rst.MainWindowHandle -outPath "target\shots\cmp-rust.png"
      "RUST_SHOT=$($rInfo.W)x$($rInfo.H)"
    }
  }
}
