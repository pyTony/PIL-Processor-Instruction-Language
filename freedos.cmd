@echo off
cd /d "%~dp0"
python tools/dos_terminal.py --engine target/release/pil.exe   --country 358 --mount DriveD --com1 console  images/FD14LITE.img::COMMAND.COM %*
