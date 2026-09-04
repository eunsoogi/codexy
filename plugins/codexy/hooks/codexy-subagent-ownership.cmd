@echo off
setlocal EnableExtensions DisableDelayedExpansion
set "event=%~1"
if /I "%event%"=="PreToolUse" goto evaluate
if /I "%event%"=="PermissionRequest" goto evaluate
set "event=PreToolUse"
:evaluate
set "runtime="
set "runtime_args="
if exist "%SystemRoot%\py.exe" (
  set "runtime=%SystemRoot%\py.exe"
  set "runtime_args=-3"
)
if not defined runtime for /f "usebackq delims=" %%I in (`"%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -Command "$ErrorActionPreference='SilentlyContinue'; $cwd=[IO.Path]::GetFullPath((Get-Location).Path); foreach($dir in $env:PATH.Split(';')) { if(-not $dir -or -not [IO.Path]::IsPathRooted($dir) -or -not (Test-Path -LiteralPath $dir -PathType Container)) { continue }; foreach($item in (Get-ChildItem -LiteralPath $dir -Filter 'py*.exe' -File)) { $candidate=[IO.Path]::GetFullPath($item.FullName); if([IO.Path]::GetDirectoryName($candidate) -ieq $cwd) { continue }; $candidate; exit 0 } }; exit 1"`) do if not defined runtime set "runtime=%%I"
if not defined runtime goto runtime_deny
if not exist "%runtime%" goto runtime_deny
"%runtime%" %runtime_args% -I -B "%~dp0codexy-subagent-ownership.py" --event "%event%"
:evaluated
set "status=%errorlevel%"
if "%status%"=="0" exit /b 0
:runtime_deny
if /I "%event%"=="PermissionRequest" goto permission_deny
echo {"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"CODEXY_SUBAGENT_OWNERSHIP_RUNTIME: Codexy policy MUST NOT execute this operation."}}
exit /b 0
:permission_deny
echo {"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"CODEXY_SUBAGENT_OWNERSHIP_RUNTIME: Codexy policy MUST NOT execute this operation."}}}
exit /b 0
