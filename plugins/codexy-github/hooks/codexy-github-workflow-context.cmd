@echo off
setlocal EnableExtensions DisableDelayedExpansion
"%SystemRoot%\System32\findstr.exe" /i /l /c:"github" /c:"issue" /c:"pull request" /c:"pull-request" /c:"pullrequest" /c:"review" /c:"merge" >nul
if errorlevel 2 exit /b %errorlevel%
if errorlevel 1 exit /b 0
echo {"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"Codexy GitHub workflow is installed. Use $git-workflow; its package-owned generic admission hooks are active."}}
exit /b 0
