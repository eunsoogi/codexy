@echo off
setlocal EnableExtensions DisableDelayedExpansion
set "event=%~1"
if /I "%event%"=="PreToolUse" goto evaluate
if /I "%event%"=="PermissionRequest" goto evaluate
set "event=PreToolUse"
:evaluate
set "CODEXY_SUBAGENT_SCRIPT=%~dp0codexy-subagent-ownership.py"
set "CODEXY_SUBAGENT_EVENT=%event%"
set "runtime=%SystemRoot%\py.exe"
set "runtime_args=-3"
if exist "%runtime%" goto invoke
set "runtime="
set "runtime_args="
for /f "delims=" %%I in ('"%SystemRoot%\System32\where.exe" py*.exe') do if not defined runtime if exist "%%~fI" if /I not "%%~dpI"=="%CD%\" if /I "%%~dpI"=="%SystemRoot%\" set "runtime=%%~fI"
if defined runtime set "runtime_args=-3"
if defined runtime goto invoke
for /f "delims=" %%I in ('"%SystemRoot%\System32\where.exe" pyth^on.exe') do if not defined runtime if exist "%%~fI" if /I not "%%~dpI"=="%CD%\" if /I "%%~dpI"=="%SystemRoot%\" set "runtime=%%~fI"
if defined runtime goto invoke
goto runtime_deny
:invoke
if not exist "%runtime%" goto runtime_deny
set "CODEXY_HOOK_SILENT=1"
"%runtime%" %runtime_args% -I -B "%CODEXY_SUBAGENT_SCRIPT%" --event "%CODEXY_SUBAGENT_EVENT%"
set "status=%errorlevel%"
if "%status%"=="0" exit /b 0
:runtime_deny
if /I "%event%"=="PermissionRequest" goto permission_deny
echo {"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"CODEXY_SUBAGENT_OWNERSHIP_RUNTIME: Codexy policy MUST NOT execute this operation."}}
exit /b 0
:permission_deny
echo {"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"CODEXY_SUBAGENT_OWNERSHIP_RUNTIME: Codexy policy MUST NOT execute this operation."}}}
exit /b 0
