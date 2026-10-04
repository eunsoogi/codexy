@echo off
setlocal EnableExtensions DisableDelayedExpansion
set "event=%~1"
@rem Accept configured events; unknown input defaults to PreToolUse.
if /I "%event%"=="PreToolUse" goto evaluate
if /I "%event%"=="PermissionRequest" goto evaluate
set "event=PreToolUse"
:evaluate
@rem Use the system Python launcher instead of a PATH-selected interpreter.
set "runtime=%SystemRoot%\py.exe"
if not exist "%runtime%" goto runtime_deny
set "CODEXY_HOOK_SILENT=1"
"%runtime%" -3 -I -B "%~dp0codexy-child-thread-creation.py" --event "%event%"
@rem Only successful policy output skips the event-specific denial.
set "status=%errorlevel%"
if "%status%"=="0" exit /b 0
:runtime_deny
if /I "%event%"=="PermissionRequest" goto permission_deny
echo {"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"CODEXY_CHILD_THREAD_CREATION_RUNTIME: Codexy policy MUST NOT execute this operation."}}
exit /b 0
:permission_deny
echo {"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"CODEXY_CHILD_THREAD_CREATION_RUNTIME: Codexy policy MUST NOT execute this operation."}}}
exit /b 0
