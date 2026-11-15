@echo off
setlocal
pushd "%~dp0"
if not exist target mkdir target
mkdir "target\pil-build-in-progress" 2>nul
if errorlevel 1 (
  echo A build is already running, or its lock remains after an interrupted build.
  echo If no build is running, remove the empty target\pil-build-in-progress folder.
  popd
  exit /b 1
)
cargo build --release --locked
set "buildResult=%errorlevel%"
rmdir "target\pil-build-in-progress"
if not "%buildResult%"=="0" echo Build failed. The browser will reject an out-of-date runtime.
popd
exit /b %buildResult%
