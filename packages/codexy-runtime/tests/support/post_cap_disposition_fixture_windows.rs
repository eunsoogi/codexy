use std::{fs, path::Path};

use crate::support::TestResult;

pub(crate) fn write_gh_companion(bin: &Path) -> TestResult<()> {
    fs::write(
        bin.join("gh.cmd"),
        r#"@echo off
setlocal EnableExtensions DisableDelayedExpansion
if /I "%~1 %~2"=="pr view" (
  type "%CODEXY_TEST_CI_RESPONSE%"
  exit /b 0
)
if /I "%~2"=="graphql" (
  type "%CODEXY_TEST_MAINTAINER_RESPONSE%"
  exit /b 0
)
if /I "%~1"=="api" if /I "%~2"=="--paginate" (
  echo(%~4| findstr.exe /C:"check-runs?per_page=100" >nul
  if not errorlevel 1 (
    type "%CODEXY_TEST_EXPECTED_CHECKS_RESPONSE%"
    exit /b 0
  )
  echo(%~4| findstr.exe /C:"check-suites?per_page=100" >nul
  if not errorlevel 1 (
    type "%CODEXY_TEST_CHECK_SUITES_RESPONSE%"
    exit /b 0
  )
)
if /I "%~1"=="api" (
  echo(%~2| findstr.exe /C:"/protection" >nul
  if not errorlevel 1 (
    type "%CODEXY_TEST_REQUIRED_STATUS_RESPONSE%"
    exit /b 0
  )
)
>&2 echo unexpected gh fixture invocation: %*
exit /b 1
"#,
    )?;
    Ok(())
}
