@echo off
@rem Keep this server's public entrypoint on the shared native dispatcher.
"%~dp0codexy-mcp-devtools.exe" lsp %*
@rem Propagate the dispatcher exit status to the caller.
exit /b %ERRORLEVEL%
