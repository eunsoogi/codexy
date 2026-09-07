@echo off
setlocal EnableExtensions DisableDelayedExpansion
set "event=%~1"
set "kind=%~2"
if /I not "%event%"=="PreToolUse" if /I not "%event%"=="PermissionRequest" set "event=PreToolUse"
if /I not "%kind%"=="issue" if /I not "%kind%"=="pr" if /I not "%kind%"=="merge" if /I not "%kind%"=="shell" if /I not "%kind%"=="nested" set "kind=shell"
py -3 -I -B -c "import subprocess,sys; p=subprocess.run([sys.executable,'-I','-B',sys.argv[1],*sys.argv[2:]],capture_output=True); sys.stdout.buffer.write(p.stdout if p.returncode==0 else b''); sys.stderr.buffer.write(p.stderr); raise SystemExit(p.returncode)" "%~dp0codexy-title-check.py" --event "%event%" --kind "%kind%" 2>nul
set "status=%errorlevel%"
if "%status%"=="0" exit /b 0
if /I "%event%"=="PermissionRequest" goto permission_deny
echo {"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"CODEXY_TITLE_CHECK_RUNTIME: Codexy title validation could not run."}}
exit /b 0
:permission_deny
echo {"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"CODEXY_TITLE_CHECK_RUNTIME: Codexy title validation could not run."}}
exit /b 0
