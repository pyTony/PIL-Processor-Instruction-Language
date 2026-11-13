# Run the existing Python shadow builder and expose its resulting engine in
# this PowerShell process. Python cannot modify the environment of its parent.
#
# Usage:
#   .\tools\build_dos_shadow.ps1 PROGRAM --verify
#   python tools/dos_terminal.py --engine $env:SHADOW_SERVICE --headless PROGRAM
#
# All arguments are forwarded unchanged to build_dos_shadow.py, including
# --output-dir, --steps, --sites and --keys.
$ErrorActionPreference = 'Stop'

if ($args.Count -eq 0) {
    throw 'Usage: .\tools\build_dos_shadow.ps1 PROGRAM.COM|PROGRAM.EXE [--verify] [other builder options]'
}

$builder = Join-Path $PSScriptRoot 'build_dos_shadow.py'
$buildOutput = @()

# Tee-Object preserves the builder's normal stdout while capturing the exact
# published binary path. Warnings and progress on stderr stay visible.
& python $builder @args | Tee-Object -Variable buildOutput
$buildExitCode = $LASTEXITCODE
if ($buildExitCode -ne 0) {
    throw "DOS shadow build failed (exit code $buildExitCode); SHADOW_SERVICE was not changed."
}

$prefix = 'Shadow service:'
$serviceLine = @($buildOutput) |
    Where-Object { $_.StartsWith($prefix, [System.StringComparison]::Ordinal) } |
    Select-Object -Last 1

if (-not $serviceLine) {
    throw 'Build completed but did not report a Shadow service path; SHADOW_SERVICE was not changed.'
}

$path = $serviceLine.Substring($prefix.Length).Trim()
if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
    throw "Built shadow engine does not exist at '$path'; SHADOW_SERVICE was not changed."
}

# Environment variables are process-wide, so the value survives returning
# from this script, without dot-sourcing. It lasts until this shell closes.
$env:SHADOW_SERVICE = (Resolve-Path -LiteralPath $path).ProviderPath
Write-Host "SHADOW_SERVICE=$env:SHADOW_SERVICE"
