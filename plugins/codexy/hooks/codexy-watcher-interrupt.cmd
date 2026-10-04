@echo off
@rem Ignore unrelated events because this wrapper handles only watcher wait lifecycle signals.
setlocal EnableExtensions DisableDelayedExpansion
set "event=%~1"
if /I not "%event%"=="PreToolUse" if /I not "%event%"=="Interrupt" if /I not "%event%"=="UserPromptSubmit" exit /b 0
set "runtime=%SystemRoot%\py.exe"
@rem The Windows Python launcher is optional; absence must leave tool execution unaffected.
if not exist "%runtime%" exit /b 0
@rem Use isolated Python without bytecode writes, then keep this advisory hook non-blocking.
"%runtime%" -3 -I -B "%~dp0codexy_watcher_interrupt.py" --event "%event%"
exit /b 0
