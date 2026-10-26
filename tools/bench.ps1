param([int]$Iterations = 1000, [int]$Samples = 3)
$ErrorActionPreference = 'Stop'
& "$PSScriptRoot/build.ps1" -Release
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    $buildRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'target' }
    & "$buildRoot/release/pil-proof.exe" --bench $Iterations $Samples
    if ($LASTEXITCODE -ne 0) { throw 'benchmark failed' }
} finally { Pop-Location }
