param([switch]$Release)
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    $profile = 'debug'
    if ($Release) { $profile = 'release'; cargo build --locked --release }
    else { cargo build --locked }
    if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
    $buildRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'target' }
    New-Item -ItemType Directory -Force "$buildRoot/programs" | Out-Null
    & "$buildRoot/$profile/pil-proof.exe" --write-selftest "$buildRoot/programs/selftest.bin"
    if ($LASTEXITCODE -ne 0) { throw 'selftest image generation failed' }
    & "$buildRoot/$profile/pil-proof.exe" --write-hello "$buildRoot/programs/hello.bin"
    if ($LASTEXITCODE -ne 0) { throw 'hello image generation failed' }
    & "$buildRoot/$profile/pil-proof.exe" --write-factorials "$buildRoot/programs/factorials.bin"
    if ($LASTEXITCODE -ne 0) { throw 'factorial image generation failed' }
    & "$buildRoot/$profile/pil-proof.exe" --write-loop "$buildRoot/programs/loop.bin"
    if ($LASTEXITCODE -ne 0) { throw 'loop image generation failed' }
    Write-Output "Built $buildRoot/$profile/pil-proof.exe and $buildRoot/programs/selftest.bin"
} finally { Pop-Location }
