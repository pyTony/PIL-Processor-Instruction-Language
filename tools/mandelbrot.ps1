param([int]$Size = 256, [int]$Iterations = 32)
$ErrorActionPreference = 'Stop'
& "$PSScriptRoot/build.ps1" -Release
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    $buildRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'target' }
    & "$buildRoot/release/pil-proof.exe" --mandelbrot $Size $Iterations
    if ($LASTEXITCODE -ne 0) { throw 'Mandelbrot render failed' }
} finally { Pop-Location }
