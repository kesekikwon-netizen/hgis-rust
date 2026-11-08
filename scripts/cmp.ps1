# 대조 캡처: 같은 탭을 C++/Rust 양쪽에서 찍어 나란히 저장
param(
  [string]$Name = 'map',
  [string]$RustCmd = 'tab:map'
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path $PSScriptRoot -Parent
& (Join-Path $PSScriptRoot 'snap.ps1') -Which cpp  -Out "target\shots\cmp-$Name-cpp.png"  | Out-Null
& (Join-Path $PSScriptRoot 'snap.ps1') -Which rust -Cmd $RustCmd -Out "target\shots\cmp-$Name-rust.png" | Out-Null

$a = [System.Drawing.Image]::FromFile((Join-Path $root "target\shots\cmp-$Name-cpp.png"))
$b = [System.Drawing.Image]::FromFile((Join-Path $root "target\shots\cmp-$Name-rust.png"))
$w = $a.Width + $b.Width + 12
$h = [Math]::Max($a.Height, $b.Height) + 26
$out = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($out)
$g.Clear([System.Drawing.Color]::FromArgb(30,30,30))
$g.DrawString('C++ 원본', (New-Object System.Drawing.Font('Malgun Gothic',11)), [System.Drawing.Brushes]::White, 4, 3)
$g.DrawString('Rust 현재', (New-Object System.Drawing.Font('Malgun Gothic',11)), [System.Drawing.Brushes]::White, ($a.Width + 14), 3)
$g.DrawImage($a, 0, 24, $a.Width, $a.Height)
$g.DrawImage($b, ($a.Width + 12), 24, $b.Width, $b.Height)
$g.Dispose()
$full = Join-Path $root "target\shots\cmp-$Name.png"
$out.Save($full, [System.Drawing.Imaging.ImageFormat]::Png)
$out.Dispose(); $a.Dispose(); $b.Dispose()
"saved $full (${w}x${h})"