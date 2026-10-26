param([string]$Program, [switch]$Check)
$ErrorActionPreference = 'Stop'
& "$PSScriptRoot/build.ps1"
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    $buildRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'target' }
    if ($Check) { & "$buildRoot/debug/pil-proof.exe" --selftest }
    else {
        if (!$Program) { $Program = "$buildRoot/programs/selftest.bin" }
        & "$buildRoot/debug/pil-proof.exe" --repl --load $Program
    }
    if ($LASTEXITCODE -ne 0) { throw 'pil-proof failed' }
} finally { Pop-Location }
