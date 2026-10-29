$outfile = "isa_pil_bundle.txt"
if (Test-Path $outfile) { Remove-Item $outfile }

Get-ChildItem -Path "isa/i386" -Filter "*.pil" -Recurse | Sort-Object FullName | ForEach-Object {
    "========================================" | Out-File -FilePath $outfile -Append -Encoding utf8
    "FILE: $($_.FullName.Replace((Get-Location).Path + [System.IO.Path]::DirectorySeparatorChar, ''))" | Out-File -FilePath $outfile -Append -Encoding utf8
    "========================================" | Out-File -FilePath $outfile -Append -Encoding utf8
    Get-Content $_.FullName -Raw | Out-File -FilePath $outfile -Append -Encoding utf8
    "`n" | Out-File -FilePath $outfile -Append -Encoding utf8
}

Write-Host "Valmis! Tallennettu tiedostoon: $outfile"