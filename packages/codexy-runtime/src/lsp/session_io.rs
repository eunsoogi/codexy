use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{ChildStderr, ChildStdout};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Result, bail};
use serde_json::Value;

use crate::mcp::FrameParser;

const STDERR_LIMIT: usize = 4000;

#[derive(Debug, Clone, Default)]
pub(super) struct SharedStderr(Arc<Mutex<String>>);

pub(super) fn spawn_stdout_reader(
    stdout: ChildStdout,
    tx: mpsc::Sender<Value>,
    stderr: &SharedStderr,
) {
    let stderr = stderr.clone();
    thread::spawn(move || read_stdout(stdout, &tx, &stderr));
}

pub(super) fn spawn_stderr_reader(
    stderr: ChildStderr,
    buffer: &SharedStderr,
) -> thread::JoinHandle<()> {
    let buffer = buffer.clone();
    thread::spawn(move || read_stderr(stderr, &buffer))
}

pub(super) fn stderr_text(buffer: &SharedStderr) -> String {
    buffer.0.lock().map(|text| text.clone()).unwrap_or_default()
}

pub(super) fn ensure_workspace_ready(buffer: &SharedStderr) -> Result<()> {
    let stderr = stderr_text(buffer);
    if stderr.contains("FetchWorkspaceError") {
        bail!("LSP workspace initialization failed: {stderr}");
    }
    Ok(())
}

fn read_stdout(mut stdout: ChildStdout, tx: &mpsc::Sender<Value>, stderr: &SharedStderr) {
    let mut parser = FrameParser::default();
    let mut chunk = [0_u8; 8192];
    loop {
        let read = match stdout.read(&mut chunk) {
            Ok(0) => return,
            Ok(read) => read,
            Err(error) => {
                cap_stderr(stderr, &error.to_string());
                return;
            }
        };
        parser.extend(&chunk[..read]);
        loop {
            match parser.next_frame() {
                Ok(Some(message)) => {
                    if tx.send(message).is_err() {
                        return;
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    cap_stderr(stderr, &error.to_string());
                    return;
                }
            }
        }
    }
}

fn read_stderr(mut stderr: ChildStderr, buffer: &SharedStderr) {
    let mut chunk = [0_u8; 4096];
    loop {
        let read = match stderr.read(&mut chunk) {
            Ok(0) => return,
            Ok(read) => read,
            Err(error) => {
                cap_stderr(buffer, &error.to_string());
                return;
            }
        };
        wait_for_fixture_stderr_gate();
        cap_stderr(buffer, &String::from_utf8_lossy(&chunk[..read]));
    }
}

fn wait_for_fixture_stderr_gate() {
    let Ok(address) = std::env::var("CODEXY_TEST_STDERR_GATE_ADDR") else {
        return;
    };
    let Ok(mut gate) = TcpStream::connect(address) else {
        return;
    };
    if gate.write_all(b"stderr-buffer-pending").is_err() || gate.flush().is_err() {
        return;
    }
    let mut release = [0_u8; 1];
    let _ = gate.read_exact(&mut release);
}

fn cap_stderr(buffer: &SharedStderr, text: &str) {
    let Ok(mut current) = buffer.0.lock() else {
        return;
    };
    current.push_str(text);
    if current.len() > STDERR_LIMIT {
        let start = current.len().saturating_sub(STDERR_LIMIT);
        current.drain(..start);
    }
}
