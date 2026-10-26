param([int]$Size = 256, [int]$Iterations = 32, [switch]$Compare)
$ErrorActionPreference = 'Stop'
& "$PSScriptRoot/build.ps1" -Release
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    $buildRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'target' }
    $mode = if ($Compare) { '--compare-mandelbrot' } else { '--mandelbrot' }
    & "$buildRoot/release/pil-proof.exe" $mode $Size $Iterations
    if ($LASTEXITCODE -ne 0) { throw 'Mandelbrot render failed' }
} finally { Pop-Location }
