use std::path::PathBuf;

use anyhow::Result;
use serde_json::{Value, json};

use crate::lsp::config::Server;
use crate::lsp::pathing::WorkspaceRoot;
use crate::lsp::session::LspSession;

#[derive(Debug, Clone, Copy)]
pub(super) enum LspMethod {
    DocumentSymbol,
    Definition,
    References,
    Diagnostics,
}

#[derive(Debug, Clone)]
pub(super) struct LspRequest {
    pub(super) server: Server,
    pub(super) file_path: String,
    pub(super) workspace_root: WorkspaceRoot,
    pub(super) method: LspMethod,
    pub(super) line: u64,
    pub(super) character: u64,
    pub(super) include_declaration: bool,
    pub(super) timeout_ms: u64,
}

impl LspRequest {
    pub(super) fn run(&self) -> Result<Value> {
        let mut session = LspSession::spawn(self)?;
        let output = session.run(self);
        let shutdown = session.shutdown();
        match (output, shutdown) {
            (Ok(value), _) => Ok(value),
            (Err(error), Ok(())) => Err(error),
            (Err(error), Err(shutdown_error)) => Err(error.context(shutdown_error.to_string())),
        }
    }

    pub(super) fn workspace_root_path(&self) -> PathBuf {
        self.workspace_root.0.clone()
    }
}

impl LspMethod {
    pub(super) const fn method_name(self) -> &'static str {
        match self {
            Self::DocumentSymbol => "textDocument/documentSymbol",
            Self::Definition => "textDocument/definition",
            Self::References => "textDocument/references",
            Self::Diagnostics => "textDocument/diagnostic",
        }
    }

    pub(super) fn params(self, uri: &str, request: &LspRequest) -> Value {
        match self {
            Self::Definition => json!({
                "textDocument": { "uri": uri },
                "position": { "line": request.line, "character": request.character }
            }),
            Self::References => json!({
                "textDocument": { "uri": uri },
                "position": { "line": request.line, "character": request.character },
                "context": { "includeDeclaration": request.include_declaration }
            }),
            Self::DocumentSymbol | Self::Diagnostics => json!({ "textDocument": { "uri": uri } }),
        }
    }
}

pub(super) fn supports_pull_diagnostics(initialize: &Value) -> bool {
    initialize
        .pointer("/result/capabilities/diagnosticProvider")
        .is_some()
}

pub(super) fn error_result(request: &LspRequest, error: &Value, stderr: &str) -> Value {
    json!({
        "status": "error",
        "server": { "id": request.server.id },
        "error": error,
        "stderr": stderr
    })
}
