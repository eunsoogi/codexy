use std::io::Write;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, bail};
use serde_json::{Value, json};

use crate::lsp::server_requests::server_request_response;
use crate::lsp::session::LspSession;

const POLL_MS: u64 = 10;

impl LspSession {
    pub(super) fn shutdown(&mut self) -> Result<()> {
        self.shutdown_until(None)
    }

    pub(super) fn shutdown_until(&mut self, deadline: Option<Instant>) -> Result<()> {
        match deadline {
            Some(deadline) if Instant::now() < deadline => {
                let _ = self.request_until("shutdown", &Value::Null, deadline);
                let _ = self.notification("exit", &Value::Null);
            }
            None => {
                let _ = self.request("shutdown", &Value::Null, 300);
                let _ = self.notification("exit", &Value::Null);
            }
            Some(_) => {}
        }
        let child_result = self.terminate_child();
        let stdout_result = self.join_stdout_reader();
        let stderr_result = self.join_stderr_reader();
        child_result.and(stdout_result).and(stderr_result)
    }

    pub(super) fn request(
        &mut self,
        method: &str,
        params: &Value,
        timeout_ms: u64,
    ) -> Result<Value> {
        self.request_until(
            method,
            params,
            Instant::now() + Duration::from_millis(timeout_ms),
        )
    }

    pub(super) fn request_until(
        &mut self,
        method: &str,
        params: &Value,
        deadline: Instant,
    ) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        self.write(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))?;
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
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    bail!("LSP server stdout closed before response")
                }
            }
        }
    }

    pub(super) fn notification(&mut self, method: &str, params: &Value) -> Result<()> {
        self.write(&json!({ "jsonrpc": "2.0", "method": method, "params": params }))
    }

    pub(super) fn handle_server_request(&mut self, message: &Value) -> Result<bool> {
        let Some(response) = server_request_response(message) else {
            return Ok(false);
        };
        self.write(&response)?;
        Ok(true)
    }

    fn terminate_child(&mut self) -> Result<()> {
        if self.child.try_wait()?.is_none() {
            let _ = self.child.kill();
            self.child.wait().context("wait for LSP server")?;
        }
        Ok(())
    }

    fn join_stderr_reader(&mut self) -> Result<()> {
        let Some(reader) = self.stderr_reader.take() else {
            return Ok(());
        };
        reader
            .join()
            .map_err(|_| anyhow::anyhow!("LSP stderr reader panicked"))
    }

    fn join_stdout_reader(&mut self) -> Result<()> {
        let Some(reader) = self.stdout_reader.take() else {
            return Ok(());
        };
        reader
            .join()
            .map_err(|_| anyhow::anyhow!("LSP stdout reader panicked"))
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

    pub(super) fn check_child(&mut self) -> Result<()> {
        if let Some(status) = self.child.try_wait()? {
            bail!("LSP server exited before response: {status}");
        }
        Ok(())
    }
}

impl Drop for LspSession {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        if let Some(reader) = self.stdout_reader.take() {
            let _ = reader.join();
        }
        if let Some(reader) = self.stderr_reader.take() {
            let _ = reader.join();
        }
    }
}
