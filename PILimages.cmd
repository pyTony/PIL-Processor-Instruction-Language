@echo off
setlocal
pushd "%~dp0"
"target\release\pil.exe" --dos-media "images/FD14LITE.img::COMMAND.COM" %*
if errorlevel 1 pause
popd
endlocal
