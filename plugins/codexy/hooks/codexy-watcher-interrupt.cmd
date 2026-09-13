@echo off
setlocal EnableExtensions DisableDelayedExpansion
set "event=%~1"
if /I not "%event%"=="PreToolUse" if /I not "%event%"=="Interrupt" exit /b 0
set "runtime=%SystemRoot%\py.exe"
if not exist "%runtime%" exit /b 0
"%runtime%" -3 -I -B "%~dp0codexy_watcher_interrupt.py" --event "%event%"
exit /b 0
