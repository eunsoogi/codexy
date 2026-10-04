@echo off
rem Keep runtime setup shared while this entrypoint fixes the dispatcher mode to LSP.
"%~dp0codexy-mcp-devtools.exe" lsp %*
exit /b %ERRORLEVEL%
