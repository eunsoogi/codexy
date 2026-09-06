use std::io::{self, Read as _, Write as _};
use std::net::TcpStream;
use std::path::Path;
use std::thread;
use std::time::Duration;

use anyhow::{Context as _, Result, bail};
use serde_json::{Value, json};

fn main() -> Result<()> {
    let mut server = FakeLsp::default();
    server.run()
}

#[derive(Debug, Default)]
struct FakeLsp {
    buffer: Vec<u8>,
    capture: Option<Value>,
    initialize_count: u64,
    request_count: u64,
    request_ids: Vec<Value>,
}

impl FakeLsp {
    fn run(&mut self) -> Result<()> {
        let mut chunk = [0_u8; 8192];
        loop {
            let read = io::stdin().read(&mut chunk)?;
            if read == 0 {
                return Ok(());
            }
            self.buffer.extend_from_slice(&chunk[..read]);
            while let Some(message) = self.next_frame()? {
                self.handle(&message)?;
            }
        }
    }

    fn next_frame(&mut self) -> Result<Option<Value>> {
        let Some(header_end) = self
            .buffer
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
        else {
            return Ok(None);
        };
        let header = std::str::from_utf8(&self.buffer[..header_end])?;
        let length = content_length(header)?;
        let start = header_end + 4;
        let end = start + length;
        if self.buffer.len() < end {
            return Ok(None);
        }
        let body = self.buffer[start..end].to_vec();
        self.buffer.drain(..end);
        serde_json::from_slice(&body)
            .map(Some)
            .context("parse frame")
    }

    fn handle(&mut self, message: &Value) -> Result<()> {
        match message.get("method").and_then(Value::as_str) {
            Some("initialize") => {
                self.capture_initialize(message)?;
                if let Some(error) = fixture_response_error(None) {
                    return Self::send(&json!({
                        "jsonrpc": "2.0",
                        "id": message.get("id").cloned().unwrap_or(Value::Null),
                        "error": { "code": -32001, "message": error }
                    }));
                }
                Self::send(&json!({
                    "jsonrpc": "2.0",
                    "id": message.get("id").cloned().unwrap_or(Value::Null),
                    "result": { "capabilities": if std::env::var_os("CODEXY_FAKE_LSP_NO_PULL_DIAGNOSTICS").is_none() { json!({ "diagnosticProvider": {} }) } else { json!({}) } }
                }))
            }
            Some("textDocument/didOpen") => self.capture_uri("openedUri", message),
            Some("shutdown") => {
                self.merge_capture(&json!({ "shutdownCount": 1 }))?;
                fixture_sync_point("shutdown-observed")?;
                fixture_release_stderr_gate();
                Self::send(&json!({
                    "jsonrpc": "2.0",
                    "id": message.get("id").cloned().unwrap_or(Value::Null),
                    "result": null
                }))
            }
            Some(_) if message.get("id").is_some() => {
                self.capture_request(message)?;
                fixture_delay()?;
                fixture_crash_after_request(self.request_count)?;
                if let Some(error) = fixture_response_error(Some(self.request_count)) {
                    return Self::send(&json!({
                        "jsonrpc": "2.0",
                        "id": message.get("id").cloned().unwrap_or(Value::Null),
                        "error": { "code": -32001, "message": error }
                    }));
                }
                Self::send(&json!({
                    "jsonrpc": "2.0",
                    "id": message.get("id").cloned().unwrap_or(Value::Null),
                    "result": []
                }))
            }
            _ => Ok(()),
        }
    }

    fn capture_initialize(&mut self, message: &Value) -> Result<()> {
        self.initialize_count += 1;
        if let Some(stderr) = fixture_stderr()? {
            let mut output = io::stderr().lock();
            output.write_all(stderr.as_bytes())?;
            output.flush()?;
            fixture_sync_point("stderr-flushed")?;
        }
        self.merge_capture(&json!({
            "initializeCount": self.initialize_count,
            "cwd": std::env::current_dir()?.display().to_string(),
            "rootUri": message.pointer("/params/rootUri").cloned().unwrap_or(Value::Null)
        }))
    }

    fn capture_uri(&mut self, key: &str, message: &Value) -> Result<()> {
        self.merge_capture(&json!({
            key: message.pointer("/params/textDocument/uri").cloned().unwrap_or(Value::Null)
        }))
    }

    fn capture_request(&mut self, message: &Value) -> Result<()> {
        self.request_count += 1;
        if let Some(id) = message.get("id") {
            self.request_ids.push(id.clone());
        }
        let mut patch = json!({
            "requestCount": self.request_count,
            "requestIds": self.request_ids.clone(),
            "requestUri": message.pointer("/params/textDocument/uri").cloned().unwrap_or(Value::Null)
        });
        if let Some(position) = message
            .get("params")
            .and_then(|params| params.get("position"))
        {
            patch["position"] = position.clone();
        }
        self.merge_capture(&patch)
    }

    fn merge_capture(&mut self, patch: &Value) -> Result<()> {
        let Some(capture_path) = std::env::var_os("CODEXY_FAKE_LSP_CAPTURE") else {
            return Ok(());
        };
        let mut current = self.capture.take().unwrap_or_else(|| json!({}));
        let Some(current_object) = current.as_object_mut() else {
            return Ok(());
        };
        if let Some(patch_object) = patch.as_object() {
            for (key, value) in patch_object {
                current_object.insert(key.clone(), value.clone());
            }
        }
        std::fs::write(capture_path, serde_json::to_vec_pretty(&current)?)?;
        self.capture = Some(current);
        Ok(())
    }

    fn send(payload: &Value) -> Result<()> {
        let body = serde_json::to_vec(payload)?;
        let mut stdout = io::stdout().lock();
        write!(stdout, "Content-Length: {}\r\n\r\n", body.len())?;
        stdout.write_all(&body)?;
        stdout.flush()?;
        Ok(())
    }
}
fn content_length(header: &str) -> Result<usize> {
    for line in header.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            return value.trim().parse().context("parse Content-Length");
        }
    }
    bail!("missing Content-Length")
}

fn fixture_sync_point(name: &str) -> Result<()> {
    if let Some(directory) = std::env::var_os("CODEXY_FAKE_LSP_SYNC_DIR") {
        std::fs::write(Path::new(&directory).join(name), b"ready")?;
    }
    Ok(())
}

fn fixture_response_error(request_count: Option<u64>) -> Option<String> {
    let error = std::env::var_os("CODEXY_FAKE_LSP_RESPONSE_ERROR")
        .map(|error| error.to_string_lossy().into_owned())?;
    let requested = std::env::var_os("CODEXY_FAKE_LSP_RESPONSE_ERROR_ON_REQUEST")
        .and_then(|value| value.to_string_lossy().parse::<u64>().ok());
    (requested.is_none() || requested == request_count).then_some(error)
}

fn fixture_stderr() -> Result<Option<String>> {
    let marker = std::env::var_os("CODEXY_FAKE_LSP_STDERR")
        .map(|stderr| stderr.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tail = std::env::var_os("CODEXY_FAKE_LSP_STDERR_TAIL_BYTES")
        .map(|tail| tail.to_string_lossy().parse::<usize>())
        .transpose()?
        .unwrap_or(0);
    if marker.is_empty() && tail == 0 {
        return Ok(None);
    }
    let separator = if marker.is_empty() { "" } else { "\n" };
    Ok(Some(format!("{marker}{separator}{}", "x".repeat(tail))))
}

fn fixture_delay() -> Result<()> {
    let delay = std::env::var_os("CODEXY_FAKE_LSP_DELAY_MS")
        .map(|value| value.to_string_lossy().parse::<u64>())
        .transpose()?
        .unwrap_or(0);
    thread::sleep(Duration::from_millis(delay));
    Ok(())
}

fn fixture_crash_after_request(request_count: u64) -> Result<()> {
    let crash_after = std::env::var_os("CODEXY_FAKE_LSP_CRASH_AFTER_REQUEST")
        .map(|value| value.to_string_lossy().parse::<u64>())
        .transpose()?;
    if crash_after == Some(request_count) {
        std::process::exit(17);
    }
    Ok(())
}

fn fixture_release_stderr_gate() {
    let Ok(address) = std::env::var("CODEXY_TEST_STDERR_SHUTDOWN_GATE_ADDR") else {
        return;
    };
    let Ok(mut gate) = TcpStream::connect(address) else {
        return;
    };
    let _ = gate
        .write_all(b"shutdown-observed")
        .and_then(|()| gate.flush());
}
