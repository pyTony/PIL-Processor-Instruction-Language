@echo off
cd /d "%~dp0"
rem Drive D: is the folder in PILDRIVE, or driveD in this checkout (git ignores it).
if not defined PILDRIVE set "PILDRIVE=driveD"
python tools/dos_terminal.py --engine target/release/pil.exe   --country 358 --mount "%PILDRIVE%" --com1 console  images/FD14LITE.img::COMMAND.COM %*
