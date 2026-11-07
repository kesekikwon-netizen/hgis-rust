$ErrorActionPreference = "Stop"
$installer = Join-Path $env:TEMP "vs_BuildTools.exe"
if (-not (Test-Path $installer)) {
    Write-Host "Downloading VS Build Tools..."
    Invoke-WebRequest -Uri 'https://aka.ms/vs/17/release/vs_BuildTools.exe' -OutFile $installer
}
Write-Host "Running VS Build Tools installer (passive mode)..."
$proc = Start-Process -FilePath $installer -ArgumentList @(
    '--passive',
    '--wait',
    '--norestart',
    '--add', 'Microsoft.VisualStudio.Workload.NativeDesktop',
    '--add', 'Microsoft.VisualStudio.Workload.VCTools',
    '--includeRecommended'
) -PassThru
$proc.WaitForExit()
Write-Host "Installer exit code: $($proc.ExitCode)"

# Find cl.exe
$clPath = Get-ChildItem "C:\Program Files*\Microsoft Visual Studio" -Recurse -Filter "cl.exe" -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty FullName
if ($clPath) {
    Write-Host "SUCCESS: cl.exe found at $clPath"
} else {
    Write-Host "WARNING: cl.exe not found yet. Installation may need a restart."
}
