use std::{fs, path::{Path, PathBuf}, process::{Command, Output}};

use crate::support::TestResult;

#[cfg(unix)]
#[test]
fn capture_runner_fails_closed_without_trusted_codex_receipt() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let temp = tempfile::tempdir()?;
    let observer = write_observer(temp.path(), false)?;
    let report = temp.path().join("report.json");
    let output = capture(fixture.root(), &observer, &report, None)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("--trusted-receipt is required"));
    assert!(!report.exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn capture_runner_rejects_self_asserted_json_receipt() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let temp = tempfile::tempdir()?;
    let observer = write_observer(temp.path(), false)?;
    let receipt = write_untrusted_receipt(temp.path())?;
    let report = temp.path().join("report.json");
    let output = capture(fixture.root(), &observer, &report, Some(&receipt))?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("trusted Codex task/tool receipt"));
    assert!(!report.exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn fixture_subprocess_can_never_authenticate_observation() -> TestResult {
    let fixture = crate::support::plugin_fixture()?;
    let temp = tempfile::tempdir()?;
    let observer = write_observer(temp.path(), true)?;
    let receipt = write_untrusted_receipt(temp.path())?;
    let report = temp.path().join("report.json");
    let output = capture(fixture.root(), &observer, &report, Some(&receipt))?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("trusted Codex task/tool receipt"));
    assert!(!report.exists());
    Ok(())
}

#[cfg(unix)]
fn capture(root: &Path, observer: &Path, output: &Path, receipt: Option<&Path>) -> TestResult<Output> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"));
    command.args(["--plugin-root", root.to_str().ok_or("plugin root")?])
        .args(["--repository-root", codexy_runtime::paths::repository_root().to_str().ok_or("repository root")?])
        .args(["--capture-economics", "--observer-command"]).arg(observer).args(["--output"]).arg(output);
    if let Some(receipt) = receipt { command.args(["--trusted-receipt"]).arg(receipt); }
    Ok(command.output()?)
}

#[cfg(unix)]
fn write_untrusted_receipt(directory: &Path) -> TestResult<PathBuf> {
    let path = directory.join("receipt.json");
    fs::write(&path, serde_json::json!({"schema":"codexy.codex-observation-receipt.v1","source":"fixture"}).to_string())?;
    Ok(path)
}

#[cfg(unix)]
fn write_observer(directory: &Path, synthetic: bool) -> TestResult<PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    let marker = if synthetic { "true" } else { "false" };
    let source = format!(r#"#!/bin/sh
set -eu
lane=''
nonce=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --lane-id) lane="$2"; shift 2 ;;
    --nonce) nonce="$2"; shift 2 ;;
    *) shift ;;
  esac
done
printf '{{"schema":"codexy.review-economics-capture.v1","lane_id":"%s","nonce":"%s","synthetic":{marker}}}
' "$lane" "$nonce"
"#);
    let path = directory.join("observer.sh");
    fs::write(&path, source)?;
    let mut permissions = fs::metadata(&path)?.permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&path, permissions)?;
    Ok(path)
}

fn stderr(output: &Output) -> String { String::from_utf8_lossy(&output.stderr).into_owned() }
