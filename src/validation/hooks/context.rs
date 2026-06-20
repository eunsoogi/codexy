use std::path::Path;
use std::process::{Command, ExitStatus, Output, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::display_relative;

#[cfg(unix)]
use std::io::{ErrorKind, Read};

#[cfg(unix)]
use std::os::fd::AsRawFd;

#[cfg(unix)]
use std::os::unix::process::{CommandExt, ExitStatusExt};

pub(super) fn emitted_session_start_context(
    script_path: &Path,
    required_event: &str,
    timeout_secs: u64,
) -> Result<String> {
    let output = output_with_timeout(
        script_path,
        required_event,
        Duration::from_secs(timeout_secs),
    )?;
    if !output.status.success() {
        bail!(
            "{} {required_event} hook command failed: {}",
            display_relative(script_path),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let data: Value = serde_json::from_slice(&output.stdout).with_context(|| {
        format!(
            "{} {required_event} hook output must be JSON",
            display_relative(script_path)
        )
    })?;
    let event = data
        .get("hookSpecificOutput")
        .and_then(|value| value.get("hookEventName"))
        .and_then(Value::as_str);
    if event != Some(required_event) {
        bail!(
            "{} hook output must set hookSpecificOutput.hookEventName to {required_event}",
            display_relative(script_path)
        );
    }

    data.get("hookSpecificOutput")
        .and_then(|value| value.get("additionalContext"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .with_context(|| {
            format!(
                "{} hook output must emit non-empty hookSpecificOutput.additionalContext",
                display_relative(script_path)
            )
        })
}

fn output_with_timeout(
    script_path: &Path,
    required_event: &str,
    timeout: Duration,
) -> Result<Output> {
    let mut command = Command::new(script_path);
    command
        .arg(required_event)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    command.process_group(0);

    let mut child = command
        .spawn()
        .with_context(|| format!("running {}", display_relative(script_path)))?;
    let child_id = child.id();
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    set_nonblocking(&stdout);
    set_nonblocking(&stderr);

    let start = Instant::now();
    let mut status = None;
    let mut stdout_data = Vec::new();
    let mut stderr_data = Vec::new();
    let mut terminated_group = false;
    loop {
        read_available(&mut stdout, &mut stdout_data)?;
        read_available(&mut stderr, &mut stderr_data)?;
        if status.is_none() {
            status = child.try_wait()?;
            if status.is_some() && !terminated_group {
                terminate_process_group(child_id, libc::SIGTERM);
                terminated_group = true;
            }
        }
        if status.is_some() && stdout.is_none() && stderr.is_none() {
            return Ok(Output {
                status: status.unwrap_or_else(timeout_status),
                stdout: stdout_data,
                stderr: stderr_data,
            });
        }
        if start.elapsed() >= timeout {
            terminate_process_group(child_id, libc::SIGKILL);
            if let Some(status) = status {
                return Ok(Output {
                    status,
                    stdout: stdout_data,
                    stderr: stderr_data,
                });
            }
            let _ = child.kill();
            let _ = child.wait();
            return Ok(Output {
                status: timeout_status(),
                stdout: stdout_data,
                stderr: timeout_stderr(script_path, required_event, timeout),
            });
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn terminate_process_group(child_id: u32, signal: i32) {
    #[cfg(unix)]
    {
        let process_group = -(child_id as i32);
        // SAFETY: kill(2) is called with a process-group id derived from a spawned child.
        // Errors are intentionally ignored because the group may already have exited.
        unsafe {
            let _ = libc::kill(process_group, signal);
        }
    }
}

fn read_available<T: Read>(stream: &mut Option<T>, buffer: &mut Vec<u8>) -> Result<()> {
    let Some(reader) = stream else {
        return Ok(());
    };
    let mut chunk = [0_u8; 4096];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => {
                *stream = None;
                return Ok(());
            }
            Ok(size) => buffer.extend_from_slice(&chunk[..size]),
            Err(error) if error.kind() == ErrorKind::WouldBlock => return Ok(()),
            Err(error) => return Err(error.into()),
        }
    }
}

fn set_nonblocking<T>(stream: &Option<T>)
where
    T: AsRawFd,
{
    #[cfg(unix)]
    if let Some(stream) = stream {
        let fd = stream.as_raw_fd();
        // SAFETY: fcntl(2) is called with a live pipe file descriptor owned by
        // the child output handle. Errors are ignored so normal blocking reads
        // remain the conservative fallback on unusual platforms.
        unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFL);
            if flags >= 0 {
                let _ = libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
            }
        }
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

fn timeout_status() -> ExitStatus {
    #[cfg(unix)]
    {
        ExitStatus::from_raw(124 << 8)
    }
}
