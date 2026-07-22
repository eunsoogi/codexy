use std::path::Path;
use std::process::{Child, ExitStatus, Output};
use std::time::Duration;

use anyhow::Result;

use crate::paths::display_relative;

use super::MAX_HOOK_OUTPUT_BYTES;

pub(super) fn finish_after_timeout(
    child: &mut Child,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    script_path: &Path,
    label: &str,
    timeout: Duration,
    terminate: impl FnOnce(),
) -> Result<Output> {
    terminate();
    let _ = child.kill();
    let _ = child.wait();
    Ok(Output {
        status: timeout_status(),
        stdout,
        stderr: format!(
            "{} {label} hook command timed out after {} second(s)",
            display_relative(script_path),
            timeout.as_secs()
        )
        .into_bytes(),
    })
}

pub(super) fn finish_after_output_exceeded(
    child: &mut Child,
    stdout: Vec<u8>,
    script_path: &Path,
    label: &str,
    terminate: impl FnOnce(),
) -> Result<Output> {
    terminate();
    let _ = child.kill();
    let _ = child.wait();
    Ok(Output {
        status: timeout_status(),
        stdout,
        stderr: format!(
            "{} {label} hook output exceeded {} byte limit",
            display_relative(script_path),
            MAX_HOOK_OUTPUT_BYTES
        )
        .into_bytes(),
    })
}

fn timeout_status() -> ExitStatus {
    use std::os::windows::process::ExitStatusExt as _;

    ExitStatus::from_raw(124)
}
