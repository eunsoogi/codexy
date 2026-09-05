use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, bail};
use serde_json::{Value, json};

use crate::lsp::config::{Server, select_server};
use crate::lsp::pathing::{
    WorkspaceRoot, path_resolution_root_from_args, resolve_path, workspace_root_from_args,
};
use crate::lsp::protocol::{LspMethod, LspRequest, failure_result};
use crate::lsp::session::LspSession;
use crate::lsp::tools::{numeric_integer, text_json};
use crate::mcp::ToolDef;

const MAX_REQUESTS: usize = 8;
const MAX_BATCH_MS: u64 = 60000;

pub(super) fn tool() -> ToolDef {
    ToolDef::new(
        "lsp_batch",
        "Run up to eight LSP requests in one bounded server session.",
        json!({
            "type": "object",
            "properties": {
                "root": { "type": "string" },
                "workspaceRoot": { "type": "string" },
                "server": { "type": "object" },
                "timeoutMs": { "type": "number", "minimum": 100, "maximum": 60000 },
                "deadlineMs": { "type": "number", "minimum": 1, "maximum": 60000 },
                "requests": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": 8,
                    "items": {
                        "type": "object",
                        "properties": {
                            "method": { "type": "string", "enum": ["lsp_document_symbols", "lsp_definition", "lsp_references", "lsp_diagnostics"] },
                            "path": { "type": "string" },
                            "line": { "type": "number" },
                            "character": { "type": "number" },
                            "includeDeclaration": { "type": "boolean" }
                        },
                        "required": ["method", "path"]
                    }
                }
            },
            "required": ["requests"]
        }),
    )
}

pub(super) fn call(args: &Value) -> Result<Value> {
    let items = args
        .get("requests")
        .and_then(Value::as_array)
        .context("requests must be a non-empty array")?;
    if items.is_empty() || items.len() > MAX_REQUESTS {
        bail!("requests must contain between 1 and {MAX_REQUESTS} items");
    }
    let timeout_ms = numeric_integer(args.get("timeoutMs"), "timeoutMs")?
        .unwrap_or(10000)
        .clamp(100, MAX_BATCH_MS);
    let deadline_ms =
        numeric_integer(args.get("deadlineMs"), "deadlineMs")?.unwrap_or(MAX_BATCH_MS);
    if deadline_ms == 0 || deadline_ms > MAX_BATCH_MS {
        bail!("deadlineMs must be between 1 and {MAX_BATCH_MS}");
    }

    let path_root = path_resolution_root_from_args(args).map(|root| root.0);
    let mut requests = Vec::with_capacity(items.len());
    let mut common_server: Option<Server> = None;
    let mut common_workspace: Option<PathBuf> = None;
    for item in items {
        let object = item.as_object().context("each request must be an object")?;
        for field in ["root", "workspaceRoot", "server", "timeoutMs", "deadlineMs"] {
            if object.contains_key(field) {
                bail!("request.{field} is not allowed; provide it once at batch level");
            }
        }
        let raw_path = object
            .get("path")
            .and_then(Value::as_str)
            .filter(|path| !path.is_empty())
            .context("request.path is required")?;
        let file_path = resolve_path(raw_path, path_root)?;
        ensure_path_within_root(raw_path, &file_path, path_root)?;
        let server = select_server(args, &file_path, path_root)?;
        let workspace = workspace_root_from_args(args, &file_path)?;
        if let Some(previous) = &common_server {
            if !same_server(previous, &server) {
                return text_json(&json!({
                    "status": "error",
                    "reason": "lsp_batch requests must resolve to one server"
                }));
            }
        } else {
            common_server = Some(server.clone());
        }
        let workspace_path = canonical_path(&workspace);
        if let Some(previous) = &common_workspace {
            if previous != &workspace_path {
                return text_json(&json!({
                    "status": "error",
                    "reason": "lsp_batch requests must resolve to one canonical workspace"
                }));
            }
        } else {
            common_workspace = Some(workspace_path);
        }
        requests.push(LspRequest {
            server,
            file_path,
            workspace_root: workspace,
            method: method(object.get("method"))?,
            line: numeric_integer(object.get("line"), "line")?.unwrap_or(0),
            character: numeric_integer(object.get("character"), "character")?.unwrap_or(0),
            include_declaration: object
                .get("includeDeclaration")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            timeout_ms,
        });
    }

    let server = common_server.context("batch server is missing")?;
    if !server.available {
        let results = requests.iter().map(unavailable_result).collect::<Vec<_>>();
        return text_json(&json!({ "status": "unavailable", "results": results }));
    }
    let deadline = Instant::now() + Duration::from_millis(deadline_ms);
    let mut session = match LspSession::spawn(&requests[0]) {
        Ok(session) => session,
        Err(error) => {
            let reason = error.to_string();
            let results = requests
                .iter()
                .map(|request| failure_result(request, &reason, ""))
                .collect::<Vec<_>>();
            return text_json(&json!({ "status": "error", "results": results }));
        }
    };
    let results = session.run_batch(&requests, deadline);
    session.shutdown_until(Some(deadline))?;
    let status = if results
        .iter()
        .all(|result| result.get("status").and_then(Value::as_str) == Some("ok"))
    {
        "ok"
    } else {
        "error"
    };
    text_json(&json!({ "status": status, "results": results }))
}

fn method(value: Option<&Value>) -> Result<LspMethod> {
    match value.and_then(Value::as_str) {
        Some("lsp_document_symbols") => Ok(LspMethod::DocumentSymbol),
        Some("lsp_definition") => Ok(LspMethod::Definition),
        Some("lsp_references") => Ok(LspMethod::References),
        Some("lsp_diagnostics") => Ok(LspMethod::Diagnostics),
        Some(name) => bail!("unsupported batch request method: {name}"),
        None => bail!("request.method is required"),
    }
}

fn same_server(left: &Server, right: &Server) -> bool {
    left.id == right.id
        && left.command == right.command
        && left.resolved_executable == right.resolved_executable
}

fn canonical_path(root: &WorkspaceRoot) -> PathBuf {
    root.0.canonicalize().unwrap_or_else(|_| root.0.clone())
}

fn unavailable_result(request: &LspRequest) -> Value {
    json!({
        "status": "unavailable",
        "path": request.file_path,
        "server": {
            "id": request.server.id,
            "executable": request.server.executable,
            "command": request.server.command
        },
        "reason": request.server.reason.clone().unwrap_or_else(|| "server executable unavailable".to_owned()),
        "installHints": request.server.install_hints
    })
}

fn ensure_path_within_root(raw_path: &str, file_path: &str, root: Option<&str>) -> Result<()> {
    let Some(root) = root else {
        return Ok(());
    };
    let root = Path::new(root)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(root));
    let path = Path::new(file_path)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(file_path));
    if !path.starts_with(&root) {
        bail!("request.path escapes root: {raw_path}");
    }
    Ok(())
}
