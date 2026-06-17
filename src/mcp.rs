use std::io::{self, Read, Write};

use anyhow::{Context as _, Result, bail};
use serde::Serialize;
use serde_json::{Value, json};

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
        parser.extend(&chunk[..read]);
        while let Some(message) = parser.next_frame()? {
            let response = handle_message(name, version, tools, &mut call_tool, &message);
            if let Some(response) = response {
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
        "initialize" => id.map(|id| {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": name, "version": version }
                }
            })
        }),
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
                Err(error) => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32000, "message": error.to_string() }
                }),
            }
        }),
        _ => id.map(|id| {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("Unknown method: {method}") }
            })
        }),
    }
}

#[derive(Debug, Default)]
pub(crate) struct FrameParser {
    buffer: Vec<u8>,
}

impl FrameParser {
    pub(crate) fn extend(&mut self, chunk: &[u8]) {
        self.buffer.extend_from_slice(chunk);
    }

    pub(crate) fn next_frame(&mut self) -> Result<Option<Value>> {
        if let Some(header_end) = find_header_end(&self.buffer) {
            let header = std::str::from_utf8(&self.buffer[..header_end])
                .context("MCP header is not UTF-8")?;
            if header_has_content_length(header) {
                return self.next_content_length_frame();
            }
        } else if starts_like_header_block(&self.buffer) {
            return Ok(None);
        }
        if starts_with_content_length(&self.buffer) {
            return self.next_content_length_frame();
        }
        self.next_newline_frame()
    }

    fn next_content_length_frame(&mut self) -> Result<Option<Value>> {
        let Some(header_end) = find_header_end(&self.buffer) else {
            return Ok(None);
        };
        let header =
            std::str::from_utf8(&self.buffer[..header_end]).context("MCP header is not UTF-8")?;
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
            .context("parsing MCP JSON frame")
    }

    fn next_newline_frame(&mut self) -> Result<Option<Value>> {
        let Some(line_end) = self.buffer.iter().position(|byte| *byte == b'\n') else {
            return Ok(None);
        };
        let mut line = self.buffer.drain(..=line_end).collect::<Vec<_>>();
        while matches!(line.last(), Some(b'\n' | b'\r')) {
            line.pop();
        }
        if line.is_empty() {
            return Ok(None);
        }
        serde_json::from_slice(&line)
            .map(Some)
            .context("parsing MCP newline JSON message")
    }
}

fn starts_with_content_length(buffer: &[u8]) -> bool {
    const HEADER: &[u8] = b"content-length:";
    buffer.len() >= HEADER.len()
        && buffer[..HEADER.len()]
            .iter()
            .zip(HEADER)
            .all(|(actual, expected)| actual.to_ascii_lowercase() == *expected)
}

fn starts_like_header_block(buffer: &[u8]) -> bool {
    let first_line_end = buffer
        .iter()
        .position(|byte| matches!(byte, b'\n' | b'\r'))
        .unwrap_or(buffer.len());
    let first_line = &buffer[..first_line_end];
    let Some(colon) = first_line.iter().position(|byte| *byte == b':') else {
        return false;
    };
    let name = &first_line[..colon];
    !name.is_empty()
        && name
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
}

fn header_has_content_length(header: &str) -> bool {
    header.lines().any(|line| {
        line.split_once(':')
            .is_some_and(|(name, _)| name.eq_ignore_ascii_case("content-length"))
    })
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn content_length(header: &str) -> Result<usize> {
    for line in header.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            return value
                .trim()
                .parse::<usize>()
                .context("parsing Content-Length header");
        }
    }
    bail!("Missing Content-Length header")
}

fn write_frame(payload: &Value) -> Result<()> {
    let body = serde_json::to_vec(payload)?;
    let mut stdout = io::stdout().lock();
    stdout.write_all(&body)?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}
