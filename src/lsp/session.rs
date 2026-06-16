use std::fs;
use std::io::Write;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, bail};
use serde_json::{Value, json};

use crate::lsp::pathing::{language_for_path, to_file_uri};
use crate::lsp::protocol::{LspMethod, LspRequest, error_result, supports_pull_diagnostics};
use crate::lsp::server_requests::server_request_response;
use crate::lsp::session_diagnostics::{has_publish_diagnostics, target_diagnostics};
use crate::lsp::session_io::{SharedStderr, spawn_stderr_reader, spawn_stdout_reader, stderr_text};

const POLL_MS: u64 = 10;

#[derive(Debug)]
pub(super) struct LspSession {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<Value>,
    stderr: SharedStderr,
    next_id: i64,
    notifications: Vec<Value>,
}

impl LspSession {
    pub(super) fn spawn(request: &LspRequest) -> Result<Self> {
        let command = request
            .server
            .command
            .as_ref()
            .filter(|items| !items.is_empty())
            .context("server command is missing")?;
        let executable = command.first().context("server command is missing")?;
        let workspace_root = request.workspace_root_path()?;
        let mut child = Command::new(executable)
            .args(command.iter().skip(1))
            .current_dir(&workspace_root)
            .envs(std::env::vars_os())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("spawn LSP server {}", request.server.id))?;
        let stdout = child
            .stdout
            .take()
            .context("LSP server stdout unavailable")?;
        let stderr = child
            .stderr
            .take()
            .context("LSP server stderr unavailable")?;
        let stdin = child.stdin.take().context("LSP server stdin unavailable")?;
        let (tx, rx) = mpsc::channel();
        let stderr_buffer = SharedStderr::default();
        spawn_stdout_reader(stdout, tx, &stderr_buffer);
        spawn_stderr_reader(stderr, &stderr_buffer);
        Ok(Self {
            child,
            stdin,
            rx,
            stderr: stderr_buffer,
            next_id: 1,
            notifications: Vec::new(),
        })
    }

    pub(super) fn run(&mut self, request: &LspRequest) -> Result<Value> {
        let uri = to_file_uri(&request.file_path)?;
        let text = fs::read_to_string(&request.file_path)
            .with_context(|| format!("reading {}", request.file_path))?;
        let root_uri = to_file_uri(&request.workspace_root_path()?.display().to_string())?;
        let initialize = self.request(
            "initialize",
            &json!({
                "processId": std::process::id(),
                "rootUri": root_uri,
                "capabilities": {
                    "textDocument": {
                        "documentSymbol": { "hierarchicalDocumentSymbolSupport": true },
                        "definition": { "linkSupport": true },
                        "references": {},
                        "diagnostic": {},
                        "synchronization": { "didSave": true }
                    },
                    "workspace": {}
                },
                "clientInfo": { "name": "codexy-lsp-mcp", "version": "0.1.0" }
            }),
            request.timeout_ms,
        )?;
        if let Some(error) = initialize.get("error") {
            return Ok(error_result(request, error, &self.stderr_text()));
        }
        self.notification("initialized", &json!({}))?;
        self.notification(
            "textDocument/didOpen",
            &json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_for_path(&request.file_path, &request.server),
                    "version": 1,
                    "text": text
                }
            }),
        )?;
        let mut result = Value::Null;
        if matches!(request.method, LspMethod::Diagnostics)
            && !supports_pull_diagnostics(&initialize)
        {
            self.wait_for_publish_diagnostics(&uri, request.timeout_ms)?;
        } else {
            let response = self.request(
                request.method.method_name(),
                &request.method.params(&uri, request),
                request.timeout_ms,
            )?;
            if let Some(error) = response.get("error") {
                return Ok(error_result(request, error, &self.stderr_text()));
            }
            result = response.get("result").cloned().unwrap_or(Value::Null);
        }
        Ok(json!({
            "status": "ok",
            "path": request.file_path,
            "server": { "id": request.server.id, "executable": request.server.executable },
            "result": result,
            "diagnostics": target_diagnostics(&self.notifications, &uri, request.method),
            "stderr": self.stderr_text()
        }))
    }

    pub(super) fn shutdown(&mut self) -> Result<()> {
        let _ = self.request("shutdown", &Value::Null, 300);
        let _ = self.notification("exit", &Value::Null);
        if self.child.try_wait()?.is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        Ok(())
    }

    fn request(&mut self, method: &str, params: &Value, timeout_ms: u64) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        self.write(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))?;
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            self.check_child()?;
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                bail!("Timed out waiting for {method}");
            };
            match self
                .rx
                .recv_timeout(remaining.min(Duration::from_millis(POLL_MS)))
            {
                Ok(message) => {
                    if self.handle_server_request(&message)? {
                        continue;
                    }
                    if message.get("id").and_then(Value::as_i64) == Some(id) {
                        return Ok(message);
                    }
                    self.notifications.push(message);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    bail!("LSP server stdout closed before response")
                }
            }
        }
    }

    fn notification(&mut self, method: &str, params: &Value) -> Result<()> {
        self.write(&json!({ "jsonrpc": "2.0", "method": method, "params": params }))
    }

    fn wait_for_publish_diagnostics(&mut self, uri: &str, timeout_ms: u64) -> Result<()> {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        while !has_publish_diagnostics(&self.notifications, uri) {
            self.check_child()?;
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return Ok(());
            };
            match self
                .rx
                .recv_timeout(remaining.min(Duration::from_millis(POLL_MS)))
            {
                Ok(message) => {
                    if !self.handle_server_request(&message)? {
                        self.notifications.push(message);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
            }
        }
        Ok(())
    }

    fn handle_server_request(&mut self, message: &Value) -> Result<bool> {
        let Some(response) = server_request_response(message) else {
            return Ok(false);
        };
        self.write(&response)?;
        Ok(true)
    }

    fn write(&mut self, payload: &Value) -> Result<()> {
        let body = serde_json::to_vec(payload)?;
        write!(self.stdin, "Content-Length: {}\r\n\r\n", body.len())
            .context("LSP server stdin unavailable")?;
        self.stdin
            .write_all(&body)
            .context("LSP server stdin unavailable")?;
        self.stdin.flush().context("LSP server stdin unavailable")?;
        Ok(())
    }

    fn check_child(&mut self) -> Result<()> {
        if let Some(status) = self.child.try_wait()? {
            bail!("LSP server exited before response: {status}");
        }
        Ok(())
    }

    fn stderr_text(&self) -> String {
        stderr_text(&self.stderr)
    }
}
