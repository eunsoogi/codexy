@echo off
set "plugin_root=%~dp0.."
set "bundled_runtime=%plugin_root%\runtime\codexy-mcp-watcher-windows-x86_64.exe"
if exist "%bundled_runtime%" goto bundled_runtime
where uvx >nul 2>&1
if errorlevel 1 (
  echo codexy-mcp-watcher requires uvx on PATH; install uv or provide a bundled runtime 1>&2
  exit /b 127
)
set "repo_root=%plugin_root%\..\.."
set "runtime_source=%repo_root%\packages\getcodexy"
if exist "%runtime_source%\pyproject.toml" goto local_source
uvx --from getcodexy==1.7.0 codexy-mcp-runtime watcher --plugin-root "%plugin_root%" -- %*
exit /b %ERRORLEVEL%

:local_source
uvx --from "%runtime_source%" codexy-mcp-runtime watcher --plugin-root "%plugin_root%" -- %*
exit /b %ERRORLEVEL%

:bundled_runtime
"%bundled_runtime%" %*
exit /b %ERRORLEVEL%
