use std::io::{ErrorKind, Read};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};

use crate::paths::display_relative;

use super::process_finish::{
    finish_after_output_exceeded, finish_after_timeout, terminate_process_group,
};

pub(super) const MAX_HOOK_OUTPUT_BYTES: usize = 1024 * 1024;
pub(in crate::validation::hooks) fn output_with_timeout(
    script_path: &Path,
    label: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<Output> {
    let mut command = Command::new(script_path);
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);

    let mut child = command
        .spawn()
        .with_context(|| format!("running {}", display_relative(script_path)))?;
    let child_id = child.id();
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    set_nonblocking(&stdout);
    set_nonblocking(&stderr);

    let start = Instant::now();
    let deadline = start + timeout;
    let mut status = None;
    let mut stdout_data = Vec::new();
    let mut stderr_data = Vec::new();
    let mut terminated_group = false;
    loop {
        match read_available(&mut stdout, &mut stdout_data, deadline)? {
            ReadOutcome::Open | ReadOutcome::Closed => {}
            ReadOutcome::TimedOut => {
                return finish_after_timeout(
                    &mut child,
                    child_id,
                    status,
                    stdout_data,
                    stderr_data,
                    script_path,
                    label,
                    timeout,
                );
            }
            ReadOutcome::OutputExceeded => {
                return finish_after_output_exceeded(
                    &mut child,
                    child_id,
                    stdout_data,
                    script_path,
                    label,
                );
            }
        }
        match read_available(&mut stderr, &mut stderr_data, deadline)? {
            ReadOutcome::Open | ReadOutcome::Closed => {}
            ReadOutcome::TimedOut => {
                return finish_after_timeout(
                    &mut child,
                    child_id,
                    status,
                    stdout_data,
                    stderr_data,
                    script_path,
                    label,
                    timeout,
                );
            }
            ReadOutcome::OutputExceeded => {
                return finish_after_output_exceeded(
                    &mut child,
                    child_id,
                    stdout_data,
                    script_path,
                    label,
                );
            }
        }
        if status.is_none() {
            status = child.try_wait()?;
            if status.is_some() && !terminated_group {
                terminate_process_group(child_id, libc::SIGTERM);
                terminated_group = true;
            }
        }
        if let Some(status) = status {
            let drain_deadline = Instant::now() + Duration::from_millis(20);
            match read_available(&mut stdout, &mut stdout_data, drain_deadline)? {
                ReadOutcome::Open | ReadOutcome::Closed | ReadOutcome::TimedOut => {}
                ReadOutcome::OutputExceeded => {
                    return finish_after_output_exceeded(
                        &mut child,
                        child_id,
                        stdout_data,
                        script_path,
                        label,
                    );
                }
            }
            match read_available(&mut stderr, &mut stderr_data, drain_deadline)? {
                ReadOutcome::Open | ReadOutcome::Closed | ReadOutcome::TimedOut => {}
                ReadOutcome::OutputExceeded => {
                    return finish_after_output_exceeded(
                        &mut child,
                        child_id,
                        stdout_data,
                        script_path,
                        label,
                    );
                }
            }
            return Ok(Output {
                status,
                stdout: stdout_data,
                stderr: stderr_data,
            });
        }
        if start.elapsed() >= timeout {
            return finish_after_timeout(
                &mut child,
                child_id,
                status,
                stdout_data,
                stderr_data,
                script_path,
                label,
                timeout,
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

enum ReadOutcome {
    Open,
    Closed,
    TimedOut,
    OutputExceeded,
}

fn read_available<T: Read>(
    stream: &mut Option<T>,
    buffer: &mut Vec<u8>,
    deadline: Instant,
) -> Result<ReadOutcome> {
    let Some(reader) = stream else {
        return Ok(ReadOutcome::Closed);
    };
    let mut chunk = [0_u8; 4096];
    loop {
        if Instant::now() >= deadline {
            return Ok(ReadOutcome::TimedOut);
        }
        let remaining = MAX_HOOK_OUTPUT_BYTES.saturating_sub(buffer.len());
        if remaining == 0 {
            return Ok(ReadOutcome::OutputExceeded);
        }
        let read_size = remaining.min(chunk.len());
        match reader.read(&mut chunk[..read_size]) {
            Ok(0) => {
                *stream = None;
                return Ok(ReadOutcome::Closed);
            }
            Ok(size) => {
                buffer.extend_from_slice(&chunk[..size]);
                if buffer.len() >= MAX_HOOK_OUTPUT_BYTES {
                    return Ok(ReadOutcome::OutputExceeded);
                }
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => return Ok(ReadOutcome::Open),
            Err(error) => return Err(error.into()),
        }
    }
}

fn set_nonblocking<T>(stream: &Option<T>)
where
    T: AsRawFd,
{
    if let Some(stream) = stream {
        let fd = stream.as_raw_fd();
        // SAFETY: fcntl(2) uses a live child-output pipe fd; errors keep blocking reads.
        unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFL);
            if flags >= 0 {
                let _ = libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
            }
        }
    }
}
