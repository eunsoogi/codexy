@echo off
rem Keep capsule checks in Python, which validates the capsule against a separate authority document.
python "%~dp0resumable_context_capsule.py" %*
exit /b %ERRORLEVEL%
