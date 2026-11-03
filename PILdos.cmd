@echo off
cd /d "%~dp0"
python tools/dos_terminal.py --engine target/release/pil.exe   --country 358 --mount DriveD --args "/K D:\AUTOEXEC.BAT" images/FD14LITE.img::COMMAND.COM %*
