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
if defined runtime goto invoke
set "CODEXY_SUBAGENT_SCRIPT=%~dp0codexy-subagent-ownership.py"
set "CODEXY_SUBAGENT_EVENT=%event%"
"%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -Command "$ErrorActionPreference='SilentlyContinue'; $cwd=[IO.Path]::GetFullPath((Get-Location).Path); $runtime=$null; foreach($dir in ($env:PATH -split ';')) { if(-not $dir -or -not [IO.Path]::IsPathRooted($dir) -or -not (Test-Path -LiteralPath $dir -PathType Container)) { continue }; foreach($item in Get-ChildItem -LiteralPath $dir -Filter 'py*.exe' -File -ErrorAction SilentlyContinue) { $candidate=[IO.Path]::GetFullPath($item.FullName); if([IO.Path]::GetDirectoryName($candidate) -ieq $cwd) { continue }; $runtime=$candidate; break }; if($runtime) { break } }; if(-not $runtime) { exit 1 }; & $runtime -I -B $env:CODEXY_SUBAGENT_SCRIPT --event $env:CODEXY_SUBAGENT_EVENT; exit $LASTEXITCODE"
if "%errorlevel%"=="0" exit /b 0
goto runtime_deny
:invoke
if not exist "%runtime%" goto runtime_deny
"%runtime%" %runtime_args% -I -B "%~dp0codexy-subagent-ownership.py" --event "%event%"
if "%errorlevel%"=="0" exit /b 0
:runtime_deny
if /I "%event%"=="PermissionRequest" goto permission_deny
echo {"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"CODEXY_SUBAGENT_OWNERSHIP_RUNTIME: Codexy policy MUST NOT execute this operation."}}
exit /b 0
:permission_deny
echo {"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"CODEXY_SUBAGENT_OWNERSHIP_RUNTIME: Codexy policy MUST NOT execute this operation."}}}
exit /b 0
