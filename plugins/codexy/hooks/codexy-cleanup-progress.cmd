@echo off
setlocal EnableExtensions DisableDelayedExpansion
set "event=%~1"
if /I "%event%"=="PostToolUse" goto evaluate
if /I "%event%"=="Stop" goto evaluate
if /I "%event%"=="Interrupt" goto evaluate
exit /b 0
:evaluate
set "runtime=%SystemRoot%\py.exe"
if not exist "%runtime%" goto unavailable
"%runtime%" -3 -I -B "%~dp0codexy_cleanup_progress.py" --event "%event%"
if "%errorlevel%"=="0" exit /b 0
:unavailable
echo {}
exit /b 0
