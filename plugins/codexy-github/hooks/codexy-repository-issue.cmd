@echo off
setlocal EnableExtensions DisableDelayedExpansion
set "event=%~1"
if /I "%event%"=="PreToolUse" goto evaluate
if /I "%event%"=="PermissionRequest" goto evaluate
set "event=PreToolUse"
:evaluate
set "output="
set "status=1"
for /f "usebackq delims=" %%I in (`py -3 -I -B "%~dp0codexy-repository-issue.py" --event "%event%" 2^>nul ^& echo CODEXY_STATUS_%%errorlevel%%`) do (
    if "%%I"=="CODEXY_STATUS_0" set "status=0"
    if not "%%I"=="CODEXY_STATUS_0" set "output=%%I"
)
if not "%status%"=="0" goto runtime_deny
if defined output echo(%output%
exit /b 0
:runtime_deny
if /I "%event%"=="PermissionRequest" goto permission_deny
echo {"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"CODEXY_REPOSITORY_ISSUE_RUNTIME: Codexy policy MUST NOT execute this operation."}}
exit /b 0
:permission_deny
echo {"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"CODEXY_REPOSITORY_ISSUE_RUNTIME: Codexy policy MUST NOT execute this operation."}}}
exit /b 0
