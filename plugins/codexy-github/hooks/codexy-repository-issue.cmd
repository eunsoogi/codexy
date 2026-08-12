@echo off
setlocal EnableExtensions DisableDelayedExpansion
if /I "%~1"=="PermissionRequest" goto permission
echo {"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"CODEXY_REPOSITORY_ISSUE_RUNTIME: Windows repository policy requires a supported package runtime."}}
exit /b 0
:permission
echo {"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"CODEXY_REPOSITORY_ISSUE_RUNTIME: Windows repository policy requires a supported package runtime."}}}
exit /b 0
