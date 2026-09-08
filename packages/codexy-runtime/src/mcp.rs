mod cancellation;
mod frame;

use std::io::{self, Read, Write};

use anyhow::{Context as _, Result};
use serde::Serialize;
use serde_json::{Value, json};

pub use cancellation::{CancellationToken, run_stdio_server_with_cancellation};
pub(crate) use frame::FrameParser;

pub(super) const MAX_FRAME_BYTES: usize = 1_048_576;
pub(super) const MAX_HEADER_BYTES: usize = 8_192;
pub(super) const MAX_BUFFER_BYTES: usize = MAX_FRAME_BYTES + MAX_HEADER_BYTES + 4;

#[derive(Debug, Clone, Serialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

impl ToolDef {
    #[must_use]
    pub fn new(name: &str, description: &str, input_schema: Value) -> Self {
        Self {
            name: name.to_owned(),
            description: description.to_owned(),
            input_schema,
        }
    }
}

#[must_use]
pub fn text_result(text: &str) -> Value {
    json!({ "content": [{ "type": "text", "text": text }] })
}

/// Runs a JSON-RPC MCP server over standard input and output.
///
/// # Errors
///
/// Returns an error when reading stdin, parsing MCP frames, handling a tool
/// call, or writing a response frame fails.
pub fn run_stdio_server<F>(
    name: &str,
    version: &str,
    tools: &[ToolDef],
    mut call_tool: F,
) -> Result<()>
where
    F: FnMut(&str, &Value) -> Result<Value>,
{
    let mut parser = FrameParser::default();
    let mut chunk = [0_u8; 8192];
    loop {
        let read = {
            let stdin = io::stdin();
            let mut stdin = stdin.lock();
            stdin.read(&mut chunk).context("reading MCP stdin")?
        };
        if read == 0 {
            break;
        }
        parser.extend(&chunk[..read])?;
        while let Some(message) = parser.next_frame()? {
            if let Some(response) = handle_message(name, version, tools, &mut call_tool, &message) {
                write_frame(&response)?;
            }
        }
    }
    Ok(())
}

fn handle_message<F>(
    name: &str,
    version: &str,
    tools: &[ToolDef],
    call_tool: &mut F,
    message: &Value,
) -> Option<Value>
where
    F: FnMut(&str, &Value) -> Result<Value>,
{
    let id = message.get("id").cloned();
    let method = message
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match method {
        "initialize" => id.map(|id| initialize_response(&id, name, version)),
        "notifications/initialized" => None,
        "tools/list" => {
            id.map(|id| json!({ "jsonrpc": "2.0", "id": id, "result": { "tools": tools } }))
        }
        "tools/call" => id.map(|id| {
            let params = message.get("params").unwrap_or(&Value::Null);
            let tool_name = params
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let arguments = params.get("arguments").unwrap_or(&Value::Null);
            match call_tool(tool_name, arguments) {
                Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                Err(error) => error_response(&id, -32000, &error.to_string()),
            }
        }),
        _ => id.map(|id| error_response(&id, -32601, &format!("Unknown method: {method}"))),
    }
}

pub(super) fn handle_control_message(
    name: &str,
    version: &str,
    tools: &[ToolDef],
    message: &Value,
) -> Option<Value> {
    let id = message.get("id").cloned();
    match message
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default()
    {
        "initialize" => id.map(|id| initialize_response(&id, name, version)),
        "notifications/initialized" => None,
        "tools/list" => {
            id.map(|id| json!({ "jsonrpc": "2.0", "id": id, "result": { "tools": tools } }))
        }
        _ => id.map(|id| {
            error_response(
                &id,
                -32601,
                &format!(
                    "Unknown method: {}",
                    message
                        .get("method")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                ),
            )
        }),
    }
}

fn initialize_response(id: &Value, name: &str, version: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": name, "version": version }
        }
    })
}

pub(super) fn error_response(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn write_frame(payload: &Value) -> Result<()> {
    let body = serde_json::to_vec(payload)?;
    let mut stdout = io::stdout().lock();
    stdout.write_all(&body)?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ToolDef, handle_control_message};
    use serde_json::json;

    #[test]
    fn control_messages_return_initialize_and_tool_list() {
        let tools = [ToolDef::new(
            "example",
            "Example",
            json!({"type": "object"}),
        )];
        let initialize = handle_control_message(
            "server",
            "1.0.0",
            &tools,
            &json!({"id": 1, "method": "initialize"}),
        )
        .expect("initialize response");
        assert_eq!(initialize["result"]["serverInfo"]["name"], "server");
        let listing = handle_control_message(
            "server",
            "1.0.0",
            &tools,
            &json!({"id": 2, "method": "tools/list"}),
        )
        .expect("tools/list response");
        assert_eq!(listing["result"]["tools"][0]["name"], "example");
    }
}
