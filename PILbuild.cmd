@echo off
setlocal
pushd "%~dp0"
python tools/runtime_startup.py --engine target/release/pil.exe %*
set "buildResult=%errorlevel%"
popd
exit /b %buildResult%
