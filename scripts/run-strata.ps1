# Strata(Rust) 실행 — 가장 최근에 빌드된 exe 를 골라 띄운다.
# release 빌드가 debug 보다 새로우면 release 를, 아니면 debug 를 쓴다.
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$log = Join-Path $root 'target\strata-launch.log'

function Write-Log([string]$m) {
  $line = "[{0}] {1}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss"), $m
  Add-Content -LiteralPath $log -Value $line -Encoding UTF8
}

try {
  Write-Log "start psscriptroot=$PSScriptRoot root=$root"

  $candidates = @(
    (Join-Path $root 'target\release\ka-hgis.exe'),
    (Join-Path $root 'target\debug\ka-hgis.exe')
  ) | Where-Object { Test-Path $_ }

  if (-not $candidates) {
    Write-Log "ERROR no exe"
    Add-Type -AssemblyName PresentationFramework
    [System.Windows.MessageBox]::Show(
      "Strata 실행 파일이 없습니다.`n먼저 빌드하세요:  cargo build -p strata-app --bin ka-hgis",
      'Strata', 'OK', 'Warning') | Out-Null
    exit 1
  }

  $exe = $candidates |
    ForEach-Object { Get-Item $_ } |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1

  Write-Log "picked=$($exe.FullName) built=$($exe.LastWriteTime)"

  $p = Start-Process -FilePath $exe.FullName -WorkingDirectory $root -PassThru
  Write-Log "spawned pid=$($p.Id)"
} catch {
  Write-Log "EXCEPTION: $($_.Exception.Message)"
  throw
}
