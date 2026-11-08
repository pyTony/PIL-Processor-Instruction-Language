@echo off
cd /d "%~dp0"
set "PILDOS_ENGINE=target\release\pil.exe"
if exist programs\compiled\pil-engine-v2.exe set "PILDOS_ENGINE=programs\compiled\pil-engine-v2.exe"
if defined PILDOS_CONFIG goto configured
if exist pildos.conf goto configured
rem Drive D: is the folder in PILDRIVE, or driveD in this checkout (git ignores it).
if not defined PILDRIVE set "PILDRIVE=driveD"
python tools/dos_terminal.py --engine "%PILDOS_ENGINE%" --country 358 --mount "%PILDRIVE%" --com1 console images/FD14LITE.img::COMMAND.COM %*
goto :eof
:configured
python tools/dos_terminal.py --engine "%PILDOS_ENGINE%" --com1 console %*
