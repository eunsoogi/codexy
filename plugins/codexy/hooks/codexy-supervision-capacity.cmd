@echo off
setlocal EnableExtensions DisableDelayedExpansion
set "event=%~1"
if /I "%event%"=="PreToolUse" goto evaluate
if /I "%event%"=="PostToolUse" goto evaluate
set "event=PreToolUse"
:evaluate
set "runtime=%SystemRoot%\py.exe"
if not exist "%runtime%" goto runtime_failure
"%runtime%" -3 -I -B "%~dp0codexy_supervision_capacity.py" --event "%event%"
if "%errorlevel%"=="0" exit /b 0
:runtime_failure
if /I "%event%"=="PostToolUse" goto post_failure
echo {"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"CODEXY_SUPERVISION_CAPACITY_RUNTIME: reservation policy unavailable."}}
exit /b 0
:post_failure
echo {"systemMessage":"Supervision state is unavailable; retain reservations and reconcile before dispatch."}
exit /b 0
