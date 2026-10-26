$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    & ./pil-proof.exe --repl --load programs/selftest.bin
    if ($LASTEXITCODE -ne 0) { throw 'pil-proof failed' }
} finally { Pop-Location }
