$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    & ./pil.exe --repl --load programs/selftest.bin
    if ($LASTEXITCODE -ne 0) { throw 'pil failed' }
} finally { Pop-Location }
