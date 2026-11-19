@echo off
rem Preserve the caller's directory so relative input paths keep their meaning.
where py >nul 2>nul
if errorlevel 1 goto python
py -3 "%~dp0tools\build_dine.py" %*
exit /b %errorlevel%
:python
python "%~dp0tools\build_dine.py" %*
exit /b %errorlevel%
