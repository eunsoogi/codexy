@echo off
rem Keep runtime setup shared while this entrypoint fixes the dispatcher mode to codegraph.
"%~dp0codexy-mcp-devtools.exe" codegraph %*
exit /b %ERRORLEVEL%
