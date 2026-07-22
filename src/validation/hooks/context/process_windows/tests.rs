use std::path::{Path, PathBuf};
use std::time::Duration;

use super::{MAX_HOOK_OUTPUT_BYTES, output_with_timeout};

#[test]
fn captures_successful_batch_output_after_both_pipes_close() -> anyhow::Result<()> {
    let fixture = BatchFixture::new("@echo off\r\necho ready\r\necho warning 1>&2\r\n")?;
    let output = output_with_timeout(&fixture.script, "test", &[], Duration::from_secs(5))?;

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "ready");
    assert_eq!(String::from_utf8_lossy(&output.stderr).trim(), "warning");
    Ok(())
}

#[test]
fn bounds_large_output_before_terminating_the_job() -> anyhow::Result<()> {
    let fixture = BatchFixture::new(
        "@echo off\r\n\"%SystemRoot%\\System32\\WindowsPowerShell\\v1.0\\powershell.exe\" -NoProfile -NonInteractive -Command \"[Console]::Out.Write('x' * 1048577)\"\r\n",
    )?;
    let output = output_with_timeout(&fixture.script, "test", &[], Duration::from_secs(10))?;

    assert!(!output.status.success());
    assert_eq!(output.stdout.len(), MAX_HOOK_OUTPUT_BYTES);
    assert!(String::from_utf8_lossy(&output.stderr).contains("output exceeded"));
    Ok(())
}

#[test]
fn timeout_terminates_descendants_in_the_job() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let child = write_batch(
        temp.path(),
        "child.cmd",
        "@echo off\r\n%SystemRoot%\\System32\\ping.exe -n 3 127.0.0.1 >nul\r\n> \"%~1\" echo survived\r\n",
    )?;
    let parent = write_batch(
        temp.path(),
        "parent.cmd",
        "@echo off\r\nstart \"\" /b \"%SystemRoot%\\System32\\cmd.exe\" /d /c call \"%~1\" \"%~2\"\r\n%SystemRoot%\\System32\\ping.exe -n 20 127.0.0.1 >nul\r\n",
    )?;
    let marker = temp.path().join("descendant-survived");
    let output = output_with_timeout(
        &parent,
        "test",
        &[path_arg(&child)?, path_arg(&marker)?],
        Duration::from_millis(1500),
    )?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("timed out"));
    std::thread::sleep(Duration::from_secs(3));
    assert!(!marker.exists(), "timed-out descendant escaped its job");
    Ok(())
}

#[test]
fn normal_parent_exit_still_terminates_descendants() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let child = write_batch(
        temp.path(),
        "child.cmd",
        "@echo off\r\n%SystemRoot%\\System32\\ping.exe -n 3 127.0.0.1 >nul\r\n> \"%~1\" echo survived\r\n",
    )?;
    let parent = write_batch(
        temp.path(),
        "parent.cmd",
        "@echo off\r\nstart \"\" /b \"%SystemRoot%\\System32\\cmd.exe\" /d /c call \"%~1\" \"%~2\"\r\necho finished\r\n",
    )?;
    let marker = temp.path().join("descendant-survived");
    let output = output_with_timeout(
        &parent,
        "test",
        &[path_arg(&child)?, path_arg(&marker)?],
        Duration::from_secs(5),
    )?;

    assert!(output.status.success());
    std::thread::sleep(Duration::from_secs(3));
    assert!(
        !marker.exists(),
        "completed parent left a descendant running"
    );
    Ok(())
}

struct BatchFixture {
    _temp: tempfile::TempDir,
    script: PathBuf,
}

impl BatchFixture {
    fn new(contents: &str) -> anyhow::Result<Self> {
        let temp = tempfile::tempdir()?;
        let script = write_batch(temp.path(), "hook.cmd", contents)?;
        Ok(Self {
            _temp: temp,
            script,
        })
    }
}

fn write_batch(directory: &Path, name: &str, contents: &str) -> anyhow::Result<PathBuf> {
    let path = directory.join(name);
    std::fs::write(&path, contents)?;
    Ok(path)
}

fn path_arg(path: &Path) -> anyhow::Result<&str> {
    path.to_str()
        .ok_or_else(|| anyhow::anyhow!("test path is not valid UTF-8"))
}
