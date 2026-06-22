use anyhow::{Context as _, Result, bail};
use serde_json::{Value, json};

use crate::lsp::config::{matching_servers, read_config, select_server};
use crate::lsp::pathing::{
    language_for_path, match_path_from_args, normalize_ext, resolve_path, root_from_args,
};
use crate::lsp::protocol::{LspMethod, LspRequest};
use crate::mcp::{ToolDef, text_result};

const MIN_TIMEOUT_MS: u64 = 100;
const MAX_TIMEOUT_MS: u64 = 60000;

#[must_use]
pub const fn server_name() -> &'static str {
    "codexy-lsp"
}

#[must_use]
pub fn tools() -> Vec<ToolDef> {
    let timeout = json!({
        "type": "number",
        "minimum": MIN_TIMEOUT_MS,
        "maximum": MAX_TIMEOUT_MS,
        "description": "Optional per-request timeout in milliseconds, clamped between 100 and 60000."
    });
    vec![
        ToolDef::new(
            "lsp_list_servers",
            "List Codexy LSP client server registrations and covered file extensions.",
            json!({"type":"object","properties":{}}),
        ),
        ToolDef::new(
            "lsp_for_path",
            "Return the Codexy LSP server registrations that match a file path.",
            json!({"type":"object","properties":{"path":{"type":"string"},"root":{"type":"string"},"workspaceRoot":{"type":"string"}},"required":["path"]}),
        ),
        ToolDef::new(
            "lsp_status",
            "Report the configured Codexy LSP server, PATH availability, and install hints for a file path.",
            json!({"type":"object","properties":{"path":{"type":"string"},"root":{"type":"string"},"workspaceRoot":{"type":"string"},"server":{"type":"object"}},"required":["path"]}),
        ),
        operation_tool(
            "lsp_document_symbols",
            "Open a file through the matching LSP server and request document symbols.",
            &timeout,
        ),
        operation_tool(
            "lsp_definition",
            "Open a file through the matching LSP server and request a definition at a position.",
            &timeout,
        ),
        operation_tool(
            "lsp_references",
            "Open a file through the matching LSP server and request references at a position.",
            &timeout,
        ),
        operation_tool(
            "lsp_diagnostics",
            "Open a file through the matching LSP server and request diagnostics.",
            &timeout,
        ),
    ]
}

/// Calls an LSP MCP tool by name.
///
/// # Errors
///
/// Returns an error when arguments are invalid, configured servers cannot be
/// loaded, an LSP process fails, or the tool name is unknown.
pub fn call_tool(name: &str, args: &Value) -> Result<Value> {
    match name {
        "lsp_list_servers" => text_json(&read_config()?),
        "lsp_for_path" => {
            let path = string_arg(args, "path")?;
            text_json(&matching_servers(
                &match_path_from_args(path, args)?,
                root_from_args(args),
            )?)
        }
        "lsp_status" => status_result(args),
        "lsp_document_symbols" => operation_result(args, LspMethod::DocumentSymbol),
        "lsp_definition" => operation_result(args, LspMethod::Definition),
        "lsp_references" => operation_result(args, LspMethod::References),
        "lsp_diagnostics" => operation_result(args, LspMethod::Diagnostics),
        _ => bail!("Unknown tool: {name}"),
    }
}

fn status_result(args: &Value) -> Result<Value> {
    let raw_path = string_arg(args, "path")?;
    let root = root_from_args(args);
    let file_path = resolve_path(raw_path, root)?;
    let server = select_server(args, &file_path, root)?;
    text_json(&json!({
        "path": file_path,
        "language": language_for_path(&file_path, &server),
        "extension": normalize_ext(&file_path),
        "server": {
            "id": server.id,
            "language": server.language,
            "command": server.command,
            "executable": server.executable,
            "resolvedExecutable": server.resolved_executable
        },
        "available": server.available,
        "installHints": server.install_hints,
        "reason": if server.available { Value::Null } else { json!(server.reason) },
        "readiness": readiness_payload(&server)
    }))
}

fn operation_tool(name: &str, description: &str, timeout: &Value) -> ToolDef {
    ToolDef::new(
        name,
        description,
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "root": { "type": "string" },
                "workspaceRoot": { "type": "string" },
                "line": { "type": "number" },
                "character": { "type": "number" },
                "includeDeclaration": { "type": "boolean" },
                "server": { "type": "object" },
                "timeoutMs": timeout
            },
            "required": ["path"]
        }),
    )
}

fn operation_result(args: &Value, method: LspMethod) -> Result<Value> {
    let raw_path = string_arg(args, "path")?;
    let root = root_from_args(args);
    let file_path = resolve_path(raw_path, root)?;
    let server = select_server(args, &file_path, root)?;
    if !server.available {
        return text_json(&unavailable_payload(&file_path, &server));
    }
    let request = LspRequest {
        server,
        file_path,
        workspace_root: root.map(ToOwned::to_owned),
        method,
        line: numeric_position(args.get("line"), "line")?.unwrap_or(0),
        character: numeric_position(args.get("character"), "character")?.unwrap_or(0),
        include_declaration: args
            .get("includeDeclaration")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        timeout_ms: timeout_ms(args)?,
    };
    match request.run() {
        Ok(result) => text_json(&result),
        Err(error) => text_json(&json!({
            "status": "error",
            "path": request.file_path,
            "server": { "id": request.server.id, "executable": request.server.executable },
            "reason": error.to_string(),
            "installHints": request.server.install_hints
        })),
    }
}

fn unavailable_payload(file_path: &str, server: &crate::lsp::config::Server) -> Value {
    json!({
        "status": "unavailable",
        "path": file_path,
        "server": { "id": server.id, "executable": server.executable, "command": server.command },
        "reason": server.reason.clone().unwrap_or_else(|| "server executable unavailable".to_owned()),
        "installHints": server.install_hints,
        "readiness": readiness_payload(server)
    })
}

fn readiness_payload(server: &crate::lsp::config::Server) -> Value {
    if server.available {
        return Value::Null;
    }
    let executable = server
        .executable
        .as_deref()
        .unwrap_or("the language server");
    let language = server.language.as_deref().unwrap_or("language-server");
    let reason = server.reason.as_deref().unwrap_or_default();
    if reason.contains("executable not found") {
        return json!({
            "defect": "missing-executable",
            "action": format!(
                "install {executable} or put it on PATH before relying on {language} LSP diagnostics"
            )
        });
    }
    json!({
        "defect": "unavailable-language-server",
        "action": "inspect lsp_status reason and install hints before relying on LSP diagnostics"
    })
}

fn string_arg<'a>(args: &'a Value, name: &str) -> Result<&'a str> {
    args.get(name)
        .and_then(Value::as_str)
        .filter(|item| !item.is_empty())
        .with_context(|| format!("{name} is required"))
}

fn numeric_position(value: Option<&Value>, name: &str) -> Result<Option<u64>> {
    numeric_integer(value, name)
}

fn numeric_integer(value: Option<&Value>, name: &str) -> Result<Option<u64>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if let Some(number) = value.as_u64() {
        return Ok(Some(number));
    }
    let Some(number) = value.as_f64() else {
        bail!("{name} must be a finite non-negative integer");
    };
    if !number.is_finite() || number < 0.0 || number.fract() != 0.0 {
        bail!("{name} must be a finite non-negative integer");
    }
    Ok(Some(format!("{number:.0}").parse()?))
}

fn timeout_ms(args: &Value) -> Result<u64> {
    let timeout = numeric_integer(args.get("timeoutMs"), "timeoutMs")?.unwrap_or(10000);
    Ok(timeout.clamp(100, 60000))
}

fn text_json<T: serde::Serialize>(value: &T) -> Result<Value> {
    Ok(text_result(&serde_json::to_string_pretty(value)?))
}
