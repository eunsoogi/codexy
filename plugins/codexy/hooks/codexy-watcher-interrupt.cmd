@echo off
setlocal EnableExtensions DisableDelayedExpansion
set "event=%~1"
rem Keep this allowlist in sync with hooks.json; unrelated lifecycle events exit without Python.
if /I not "%event%"=="PreToolUse" if /I not "%event%"=="Interrupt" if /I not "%event%"=="UserPromptSubmit" exit /b 0
rem Use the system Python launcher in isolated, no-bytecode mode to run the packaged sibling helper.
set "runtime=%SystemRoot%\py.exe"
if not exist "%runtime%" exit /b 0
"%runtime%" -3 -I -B "%~dp0codexy_watcher_interrupt.py" --event "%event%"
exit /b 0
