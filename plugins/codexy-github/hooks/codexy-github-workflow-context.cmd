@echo off
setlocal EnableExtensions DisableDelayedExpansion
set "matched="
for /f "delims=" %%I in ('%SystemRoot%\System32\findstr.exe /i /l /c:"github" /c:"issue" /c:"pull request" /c:"pull-request" /c:"pullrequest" /c:"review" /c:"merge"') do set "matched=1"
if not defined matched exit /b 0
echo {"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"Codexy GitHub workflow is installed. Use $git-workflow; GitHub authorization remains with the host, connector, and GitHub."}}
exit /b 0
