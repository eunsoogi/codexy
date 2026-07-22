@echo off
setlocal EnableExtensions DisableDelayedExpansion
set "event=%~1"
if /I "%event%"=="PreToolUse" goto evaluate
if /I "%event%"=="PermissionRequest" goto evaluate
set "event=PreToolUse"

:evaluate
REM Static-only contract until native Windows product execution is proven by #454.
py -3 -I -B "%~dp0codexy-admission.py" --event "%event%"
if not errorlevel 1 exit /b 0
if /I "%event%"=="PermissionRequest" goto permission_deny
echo {"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"Codexy policy: MUST NOT execute when the static admission runtime is unavailable."}}
exit /b 0

:permission_deny
echo {"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"Codexy policy: MUST NOT execute when the static admission runtime is unavailable."}}}
exit /b 0
