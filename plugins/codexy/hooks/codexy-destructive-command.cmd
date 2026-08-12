@echo off
setlocal EnableExtensions DisableDelayedExpansion
set "event=%~1"
if /I "%event%"=="PreToolUse" goto evaluate
if /I "%event%"=="PermissionRequest" goto evaluate
set "event=PreToolUse"
:evaluate
set "output="
for /f "usebackq delims=" %%I in (`py -3 -I -B -c "import os,tempfile; fd,path=tempfile.mkstemp(prefix='codexy-hook-',suffix='.out'); os.close(fd); print(path)" 2^>nul`) do set "output=%%I"
if not defined output goto discard
py -3 -I -B "%~dp0codexy-destructive-command.py" --event "%event%" > "%output%" 2>nul
set "status=%errorlevel%"
if not "%status%"=="0" goto discard
type "%output%" 2>nul
set "status=%errorlevel%"
:discard
del "%output%" >nul 2>nul
if "%status%"=="0" exit /b 0
if /I "%event%"=="PermissionRequest" goto permission_deny
echo {"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"CODEXY_DESTRUCTIVE_COMMAND_RUNTIME: Codexy policy MUST NOT execute this operation."}}
exit /b 0
:permission_deny
echo {"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"CODEXY_DESTRUCTIVE_COMMAND_RUNTIME: Codexy policy MUST NOT execute this operation."}}}
exit /b 0
