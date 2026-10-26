param([int]$Iterations = 10000, [int]$Samples = 5)
$ErrorActionPreference = 'Stop'
& "$PSScriptRoot/build.ps1" -Release
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    $buildRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'target' }
    & "$buildRoot/release/pil-proof.exe" --compare-backends $Iterations $Samples
    if ($LASTEXITCODE -ne 0) { throw 'backend comparison failed' }
} finally { Pop-Location }
