# Download large QGIS Desktop User Guide PDFs (not in git).
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Dest = Join-Path $Root "docs\vendor\qgis-manual-3.44"
New-Item -ItemType Directory -Force -Path $Dest | Out-Null

$files = @(
  @{ Name = "QGIS-3.44-DesktopUserGuide-en.pdf"; Url = "https://docs.qgis.org/3.44/pdf/en/QGIS-3.44-DesktopUserGuide-en.pdf" },
  @{ Name = "QGIS-3.44-DesktopUserGuide-ko.pdf"; Url = "https://docs.qgis.org/3.44/pdf/ko/QGIS-3.44-DesktopUserGuide-ko.pdf" }
)

foreach ($f in $files) {
  $out = Join-Path $Dest $f.Name
  if ((Test-Path $out) -and ((Get-Item $out).Length -gt 1MB)) {
    Write-Host "skip existing $($f.Name) ($([math]::Round((Get-Item $out).Length/1MB,1)) MB)"
    continue
  }
  Write-Host "GET $($f.Url)"
  Invoke-WebRequest -Uri $f.Url -OutFile $out -UseBasicParsing -TimeoutSec 600
  Write-Host "OK $out ($([math]::Round((Get-Item $out).Length/1MB,1)) MB)"
}

Write-Host "Cookbooks (in git):"
Get-ChildItem $Dest -Filter "*Cookbook*" -ErrorAction SilentlyContinue | ForEach-Object { "  $($_.Name)" }
Write-Host "See $Dest\README.md"
