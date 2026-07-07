use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{ExitStatus, Output};
use std::time::Duration;

use anyhow::Result;

use crate::paths::display_relative;

use super::process::MAX_HOOK_OUTPUT_BYTES;

pub(super) fn finish_after_timeout(
    child: &mut std::process::Child,
    child_id: u32,
    status: Option<ExitStatus>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    script_path: &Path,
    required_event: &str,
    timeout: Duration,
) -> Result<Output> {
    terminate_process_group(child_id, libc::SIGKILL);
    if let Some(status) = status {
        return Ok(Output {
            status,
            stdout,
            stderr,
        });
    }
    let _ = child.kill();
    let _ = child.wait();
    Ok(Output {
        status: timeout_status(),
        stdout,
        stderr: timeout_stderr(script_path, required_event, timeout),
    })
}

pub(super) fn finish_after_output_exceeded(
    child: &mut std::process::Child,
    child_id: u32,
    stdout: Vec<u8>,
    script_path: &Path,
    required_event: &str,
) -> Result<Output> {
    terminate_process_group(child_id, libc::SIGKILL);
    let _ = child.kill();
    let _ = child.wait();
    Ok(Output {
        status: timeout_status(),
        stdout,
        stderr: output_exceeded_stderr(script_path, required_event),
    })
}

pub(super) fn terminate_process_group(child_id: u32, signal: i32) {
    let process_group = -(child_id as i32);
    // SAFETY: kill(2) uses a process-group id from a spawned child; errors mean it already exited.
    unsafe {
        let _ = libc::kill(process_group, signal);
    }
}

fn timeout_stderr(script_path: &Path, required_event: &str, timeout: Duration) -> Vec<u8> {
    format!(
        "{} {required_event} hook command timed out after {} second(s)",
        display_relative(script_path),
        timeout.as_secs()
    )
    .into_bytes()
}

fn output_exceeded_stderr(script_path: &Path, required_event: &str) -> Vec<u8> {
    format!(
        "{} {required_event} hook output exceeded {} byte limit",
        display_relative(script_path),
        MAX_HOOK_OUTPUT_BYTES
    )
    .into_bytes()
}

fn timeout_status() -> ExitStatus {
    ExitStatus::from_raw(124 << 8)
}
