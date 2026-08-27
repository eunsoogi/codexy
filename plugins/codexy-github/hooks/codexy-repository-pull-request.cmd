@echo off
setlocal EnableExtensions DisableDelayedExpansion
set "event=%~1"
if /I "%event%"=="PreToolUse" goto evaluate
if /I "%event%"=="PermissionRequest" goto evaluate
set "event=PreToolUse"
:evaluate
rem Static fixture pairing marker: py -3 -I -B "%~dp0codexy-repository-pull-request.py" --event "%event%"
py -3 -I -B -c "import subprocess,sys; p=subprocess.run([sys.executable,'-I','-B',sys.argv[1],*sys.argv[2:]],capture_output=True); sys.stdout.buffer.write(p.stdout if p.returncode==0 else b''); sys.stderr.buffer.write(p.stderr); raise SystemExit(p.returncode)" "%~dp0codexy-repository-pull-request.py" --event "%event%" 2>nul
set "status=%errorlevel%"
if "%status%"=="0" exit /b 0
if /I "%event%"=="PermissionRequest" goto permission_deny
echo {"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"CODEXY_REPOSITORY_PULL_REQUEST_RUNTIME: Codexy policy MUST NOT execute this operation."}}
exit /b 0
:permission_deny
echo {"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"CODEXY_REPOSITORY_PULL_REQUEST_RUNTIME: Codexy policy MUST NOT execute this operation."}}}
exit /b 0
