@echo off
setlocal EnableExtensions DisableDelayedExpansion
set "event=%~1"
if /I "%event%"=="PreToolUse" goto evaluate
if /I "%event%"=="PermissionRequest" goto evaluate
set "event=PreToolUse"
:evaluate
set "runtime=%SystemRoot%\py.exe"
if not exist "%runtime%" goto runtime_deny
set "CODEXY_HOOK_SILENT=1"
"%runtime%" -3 -I -B "%~dp0codexy-thread-delivery.py" --event "%event%"
set "status=%errorlevel%"
if "%status%"=="0" exit /b 0
:runtime_deny
if /I "%event%"=="PermissionRequest" goto permission_deny
echo {"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"CODEXY_THREAD_DELIVERY_RUNTIME: Codexy policy MUST NOT execute this operation."}}
exit /b 0
:permission_deny
echo {"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"CODEXY_THREAD_DELIVERY_RUNTIME: Codexy policy MUST NOT execute this operation."}}}
exit /b 0
