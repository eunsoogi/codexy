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
const WORKSPACE_ERROR: &str = "FetchWorkspaceError";

#[derive(Debug, Default)]
struct StderrState {
    display: String,
    workspace_error_seen: bool,
    workspace_error_probe: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct SharedStderr(Arc<Mutex<StderrState>>);

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
    buffer
        .0
        .lock()
        .map(|state| state.display.clone())
        .unwrap_or_default()
}

pub(super) fn ensure_workspace_ready(buffer: &SharedStderr) -> Result<()> {
    let Ok(state) = buffer.0.lock() else {
        return Ok(());
    };
    if state.workspace_error_seen {
        bail!(
            "LSP workspace initialization failed: {WORKSPACE_ERROR}: {}",
            state.display
        );
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
                append_stderr(stderr, &error.to_string());
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
                    append_stderr(stderr, &error.to_string());
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
                append_stderr(buffer, &error.to_string());
                return;
            }
        };
        wait_for_fixture_stderr_gate();
        append_stderr(buffer, &String::from_utf8_lossy(&chunk[..read]));
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

fn append_stderr(buffer: &SharedStderr, text: &str) {
    let Ok(mut state) = buffer.0.lock() else {
        return;
    };
    observe_workspace_error(&mut state, text);
    state.display.push_str(text);
    if state.display.len() > STDERR_LIMIT {
        let start = state.display.len().saturating_sub(STDERR_LIMIT);
        state.display.drain(..start);
    }
}

fn observe_workspace_error(state: &mut StderrState, text: &str) {
    if state.workspace_error_seen {
        return;
    }
    state
        .workspace_error_probe
        .extend_from_slice(text.as_bytes());
    if state
        .workspace_error_probe
        .windows(WORKSPACE_ERROR.len())
        .any(|window| window == WORKSPACE_ERROR.as_bytes())
    {
        state.workspace_error_seen = true;
        state.workspace_error_probe.clear();
        return;
    }
    let retained = WORKSPACE_ERROR.len().saturating_sub(1);
    if state.workspace_error_probe.len() > retained {
        let start = state.workspace_error_probe.len() - retained;
        state.workspace_error_probe.drain(..start);
    }
}
