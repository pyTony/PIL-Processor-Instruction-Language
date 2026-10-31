param([int]$Iterations = 1000, [int]$Samples = 3, [switch]$Uncached)
$ErrorActionPreference = 'Stop'
& "$PSScriptRoot/build.ps1" -Release
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    $buildRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'target' }
    $benchArgs = @('--bench', $Iterations, $Samples)
    if ($Uncached) { $benchArgs += '--uncached' }
    & "$buildRoot/release/pil.exe" @benchArgs
    if ($LASTEXITCODE -ne 0) { throw 'benchmark failed' }
} finally { Pop-Location }
