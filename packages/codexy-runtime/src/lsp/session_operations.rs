use std::fs;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use serde_json::{Value, json};

use crate::lsp::pathing::{language_for_path, to_file_uri};
use crate::lsp::protocol::{
    LspMethod, LspRequest, error_result, failure_result, supports_pull_diagnostics,
};
use crate::lsp::session::LspSession;
use crate::lsp::session_diagnostics::{has_publish_diagnostics, target_diagnostics};
use crate::lsp::session_io::{ensure_workspace_ready, stderr_text};

impl LspSession {
    pub(super) fn run(&mut self, request: &LspRequest) -> Result<Value> {
        let initialize = self.initialize(
            request,
            Instant::now() + Duration::from_millis(request.timeout_ms),
        )?;
        if let Some(error) = initialize.get("error") {
            return Ok(error_result(request, error, &stderr_text(&self.stderr)));
        }
        self.run_request(
            request,
            Instant::now() + Duration::from_millis(request.timeout_ms),
            &initialize,
        )
    }

    pub(super) fn run_batch(&mut self, requests: &[LspRequest], deadline: Instant) -> Vec<Value> {
        let initialize = match self.initialize(&requests[0], deadline) {
            Ok(value) => value,
            Err(error) => {
                return requests
                    .iter()
                    .map(|request| {
                        failure_result(request, &error.to_string(), &stderr_text(&self.stderr))
                    })
                    .collect();
            }
        };
        if let Some(error) = initialize.get("error") {
            return requests
                .iter()
                .map(|request| error_result(request, error, &stderr_text(&self.stderr)))
                .collect();
        }

        let mut results = Vec::with_capacity(requests.len());
        for (index, request) in requests.iter().enumerate() {
            if Instant::now() >= deadline {
                append_unstarted(&mut results, &requests[index..], "batch deadline exceeded");
                break;
            }
            let request_deadline =
                (Instant::now() + Duration::from_millis(request.timeout_ms)).min(deadline);
            match self.run_request(request, request_deadline, &initialize) {
                Ok(mut result) => {
                    if result.get("status").and_then(Value::as_str) == Some("ok")
                        && let Err(error) = ensure_workspace_ready(&self.stderr)
                    {
                        result =
                            failure_result(request, &error.to_string(), &stderr_text(&self.stderr));
                    }
                    results.push(result);
                }
                Err(error) => {
                    let reason = error.to_string();
                    results.push(failure_result(request, &reason, &stderr_text(&self.stderr)));
                    append_unstarted(
                        &mut results,
                        &requests[index + 1..],
                        &format!("not executed: {reason}"),
                    );
                    break;
                }
            }
        }
        results
    }

    fn initialize(&mut self, request: &LspRequest, deadline: Instant) -> Result<Value> {
        let root_uri = to_file_uri(&request.workspace_root_path().display().to_string())?;
        let initialize = self.request_until(
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
                "clientInfo": { "name": "codexy-lsp-mcp", "version": env!("CARGO_PKG_VERSION") }
            }),
            deadline,
        )?;
        if initialize.get("error").is_none() {
            self.notification("initialized", &json!({}))?;
        }
        Ok(initialize)
    }

    fn run_request(
        &mut self,
        request: &LspRequest,
        deadline: Instant,
        initialize: &Value,
    ) -> Result<Value> {
        let uri = to_file_uri(&request.file_path)?;
        let text = fs::read_to_string(&request.file_path)
            .with_context(|| format!("reading {}", request.file_path))?;
        let notification_start = self.notifications.len();
        self.open_document(request, &uri, &text)?;
        let mut result = Value::Null;
        if matches!(request.method, LspMethod::Diagnostics)
            && !supports_pull_diagnostics(initialize)
        {
            self.wait_for_publish_diagnostics(&uri, notification_start, deadline)?;
        } else {
            let response = self.request_until(
                request.method.method_name(),
                &request.method.params(&uri, request),
                deadline,
            )?;
            if let Some(error) = response.get("error") {
                return Ok(error_result(request, error, &stderr_text(&self.stderr)));
            }
            result = response.get("result").cloned().unwrap_or(Value::Null);
        }
        Ok(json!({
            "status": "ok",
            "path": request.file_path,
            "server": { "id": request.server.id, "executable": request.server.executable },
            "result": result,
            "diagnostics": target_diagnostics(
                &self.notifications[notification_start..],
                &uri,
                request.method,
            ),
            "stderr": stderr_text(&self.stderr)
        }))
    }

    fn open_document(&mut self, request: &LspRequest, uri: &str, text: &str) -> Result<()> {
        let version = self.opened_documents.get(uri).copied().unwrap_or(0) + 1;
        if version == 1 {
            self.notification(
                "textDocument/didOpen",
                &json!({ "textDocument": {
                    "uri": uri,
                    "languageId": language_for_path(&request.file_path, &request.server),
                    "version": version,
                    "text": text
                }}),
            )?;
        } else {
            self.notification(
                "textDocument/didChange",
                &json!({ "textDocument": { "uri": uri, "version": version },
                    "contentChanges": [{ "text": text }] }),
            )?;
        }
        self.opened_documents.insert(uri.to_owned(), version);
        Ok(())
    }

    fn wait_for_publish_diagnostics(
        &mut self,
        uri: &str,
        notification_start: usize,
        deadline: Instant,
    ) -> Result<()> {
        while !has_publish_diagnostics(&self.notifications[notification_start..], uri) {
            self.check_child()?;
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                anyhow::bail!("Timed out waiting for diagnostics");
            };
            match self
                .rx
                .recv_timeout(remaining.min(Duration::from_millis(10)))
            {
                Ok(message) => {
                    if !self.handle_server_request(&message)? {
                        self.notifications.push(message);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    anyhow::bail!("LSP server stdout closed before diagnostics")
                }
            }
        }
        Ok(())
    }
}

fn append_unstarted(results: &mut Vec<Value>, requests: &[LspRequest], reason: &str) {
    for request in requests {
        results.push(failure_result(request, reason, ""));
    }
}
