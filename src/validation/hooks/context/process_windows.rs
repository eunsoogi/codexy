use std::io::{ErrorKind, Read};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, bail};

use crate::paths::display_relative;

#[path = "process_windows/job.rs"]
mod job;
use job::Job;
#[path = "process_windows/finish.rs"]
mod finish;
use finish::{finish_after_output_exceeded, finish_after_timeout};

pub(super) const MAX_HOOK_OUTPUT_BYTES: usize = 1024 * 1024;

pub(in crate::validation::hooks) fn output_with_timeout(
    script_path: &Path,
    label: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<Output> {
    let isolated_home = tempfile::tempdir().context("creating isolated hook validation home")?;
    let system_root = std::env::var_os("SystemRoot").context("Windows SystemRoot unavailable")?;
    let system_path = Path::new(&system_root).join("System32");
    let mut command = Command::new(script_path);
    command
        .args(args)
        .env_clear()
        .env("USERPROFILE", isolated_home.path())
        .env("CODEX_HOME", isolated_home.path().join(".codex"))
        .env("SystemRoot", system_root)
        .env("PATH", system_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .with_context(|| format!("running {}", display_relative(script_path)))?;
    let mut job = match Job::assign(&child) {
        Ok(job) => Some(job),
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };
    let (sender, receiver) = mpsc::sync_channel(16);
    spawn_reader(
        child.stdout.take().context("capturing hook stdout")?,
        Stream::Stdout,
        sender.clone(),
    );
    spawn_reader(
        child.stderr.take().context("capturing hook stderr")?,
        Stream::Stderr,
        sender,
    );

    let deadline = Instant::now() + timeout;
    let mut output = CollectedOutput::default();
    let mut closed = 0;
    loop {
        match receive(&receiver, &mut output, &mut closed, deadline)? {
            Receive::OutputExceeded => {
                return finish_after_output_exceeded(
                    &mut child,
                    output.stdout,
                    script_path,
                    label,
                    || drop(job.take()),
                );
            }
            Receive::Open | Receive::Closed => {}
            Receive::TimedOut => {
                return finish_after_timeout(
                    &mut child,
                    output.stdout,
                    output.stderr,
                    script_path,
                    label,
                    timeout,
                    || drop(job.take()),
                );
            }
        }
        if let Some(status) = child.try_wait()? {
            drop(job.take());
            drain_after_termination(&receiver, &mut output, &mut closed)?;
            if output.exceeded() {
                return finish_after_output_exceeded(
                    &mut child,
                    output.stdout,
                    script_path,
                    label,
                    || {},
                );
            }
            return Ok(Output {
                status,
                stdout: output.stdout,
                stderr: output.stderr,
            });
        }
    }
}

fn drain_after_termination(
    receiver: &Receiver<ReaderEvent>,
    output: &mut CollectedOutput,
    closed: &mut u8,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_millis(100);
    while *closed < 2 && Instant::now() < deadline {
        match receive(receiver, output, closed, deadline)? {
            Receive::Open | Receive::Closed | Receive::TimedOut => {}
            Receive::OutputExceeded => return Ok(()),
        }
    }
    Ok(())
}

fn receive(
    receiver: &Receiver<ReaderEvent>,
    output: &mut CollectedOutput,
    closed: &mut u8,
    deadline: Instant,
) -> Result<Receive> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Ok(Receive::TimedOut);
    }
    match receiver.recv_timeout(remaining.min(Duration::from_millis(20))) {
        Ok(ReaderEvent::Chunk(stream, bytes)) => {
            output.extend(stream, &bytes);
            Ok(if output.exceeded() {
                Receive::OutputExceeded
            } else {
                Receive::Open
            })
        }
        Ok(ReaderEvent::Closed) => {
            *closed += 1;
            Ok(Receive::Closed)
        }
        Ok(ReaderEvent::Failed(error)) => Err(error.into()),
        Err(mpsc::RecvTimeoutError::Timeout) => Ok(Receive::Open),
        Err(mpsc::RecvTimeoutError::Disconnected) => bail!("hook output reader disconnected"),
    }
}

fn spawn_reader(
    mut reader: impl Read + Send + 'static,
    stream: Stream,
    sender: mpsc::SyncSender<ReaderEvent>,
) {
    std::thread::spawn(move || {
        let mut buffer = [0_u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    let _ = sender.send(ReaderEvent::Closed);
                    return;
                }
                Ok(size) => {
                    if sender
                        .send(ReaderEvent::Chunk(stream, buffer[..size].to_vec()))
                        .is_err()
                    {
                        return;
                    }
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(error) => {
                    let _ = sender.send(ReaderEvent::Failed(error));
                    return;
                }
            }
        }
    });
}

#[derive(Clone, Copy)]
enum Stream {
    Stdout,
    Stderr,
}

enum ReaderEvent {
    Chunk(Stream, Vec<u8>),
    Closed,
    Failed(std::io::Error),
}

enum Receive {
    Open,
    Closed,
    TimedOut,
    OutputExceeded,
}

#[derive(Default)]
struct CollectedOutput {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl CollectedOutput {
    fn extend(&mut self, stream: Stream, bytes: &[u8]) {
        match stream {
            Stream::Stdout => self.stdout.extend_from_slice(bytes),
            Stream::Stderr => self.stderr.extend_from_slice(bytes),
        }
    }

    fn exceeded(&self) -> bool {
        self.stdout.len() >= MAX_HOOK_OUTPUT_BYTES || self.stderr.len() >= MAX_HOOK_OUTPUT_BYTES
    }
}
