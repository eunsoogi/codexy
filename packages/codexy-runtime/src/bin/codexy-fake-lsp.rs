use std::io::{self, Read as _, Write as _};
use std::net::TcpStream;
use std::path::Path;

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
                if let Some(error) = Self::fixture_response_error() {
                    return Self::send(&json!({
                        "jsonrpc": "2.0",
                        "id": message.get("id").cloned().unwrap_or(Value::Null),
                        "error": { "code": -32001, "message": error }
                    }));
                }
                Self::send(&json!({
                    "jsonrpc": "2.0",
                    "id": message.get("id").cloned().unwrap_or(Value::Null),
                    "result": { "capabilities": { "diagnosticProvider": {} } }
                }))
            }
            Some("textDocument/didOpen") => self.capture_uri("openedUri", message),
            Some("shutdown") => {
                Self::fixture_sync_point("shutdown-observed")?;
                Self::release_fixture_stderr_gate();
                Self::send(&json!({
                    "jsonrpc": "2.0",
                    "id": message.get("id").cloned().unwrap_or(Value::Null),
                    "result": null
                }))
            }
            Some(_) if message.get("id").is_some() => {
                self.capture_request(message)?;
                if let Some(error) = Self::fixture_response_error() {
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
        if let Some(stderr) = Self::fixture_stderr()? {
            let mut output = io::stderr().lock();
            output.write_all(stderr.as_bytes())?;
            output.flush()?;
            Self::fixture_sync_point("stderr-flushed")?;
        }
        self.merge_capture(&json!({
            "cwd": std::env::current_dir()?.display().to_string(),
            "rootUri": message.pointer("/params/rootUri").cloned().unwrap_or(Value::Null)
        }))
    }

    fn fixture_sync_point(name: &str) -> Result<()> {
        let Some(directory) = std::env::var_os("CODEXY_FAKE_LSP_SYNC_DIR") else {
            return Ok(());
        };
        std::fs::write(Path::new(&directory).join(name), b"ready")?;
        Ok(())
    }

    fn fixture_response_error() -> Option<String> {
        std::env::var_os("CODEXY_FAKE_LSP_RESPONSE_ERROR")
            .map(|error| error.to_string_lossy().into_owned())
    }

    fn fixture_stderr() -> Result<Option<String>> {
        let marker = std::env::var_os("CODEXY_FAKE_LSP_STDERR")
            .map(|stderr| stderr.to_string_lossy().into_owned())
            .unwrap_or_default();
        let tail = match std::env::var_os("CODEXY_FAKE_LSP_STDERR_TAIL_BYTES") {
            Some(tail) => tail
                .to_string_lossy()
                .parse::<usize>()
                .context("parse fake LSP stderr tail length")?,
            None => 0,
        };
        if marker.is_empty() && tail == 0 {
            return Ok(None);
        }
        let mut stderr = marker;
        if !stderr.is_empty() {
            stderr.push('\n');
        }
        stderr.extend(std::iter::repeat_n('x', tail));
        Ok(Some(stderr))
    }

    fn release_fixture_stderr_gate() {
        let Ok(address) = std::env::var("CODEXY_TEST_STDERR_SHUTDOWN_GATE_ADDR") else {
            return;
        };
        let Ok(mut gate) = TcpStream::connect(address) else {
            return;
        };
        let _ = gate.write_all(b"shutdown-observed");
        let _ = gate.flush();
    }

    fn capture_uri(&mut self, key: &str, message: &Value) -> Result<()> {
        self.merge_capture(&json!({
            key: message.pointer("/params/textDocument/uri").cloned().unwrap_or(Value::Null)
        }))
    }

    fn capture_request(&mut self, message: &Value) -> Result<()> {
        let mut patch = json!({
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
