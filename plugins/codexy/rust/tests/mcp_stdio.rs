use std::io::{Read as _, Write as _};
use std::process::{Child, Command, Stdio};

use serde_json::{Value, json};

struct McpClient {
    child: Child,
    buffer: Vec<u8>,
}

impl McpClient {
    fn spawn(binary: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let child = Command::new(binary)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        Ok(Self {
            child,
            buffer: Vec::new(),
        })
    }

    fn send(&mut self, payload: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let body = serde_json::to_vec(&payload)?;
        let stdin = self.child.stdin.as_mut().ok_or("missing child stdin")?;
        write!(stdin, "Content-Length: {}\r\n\r\n", body.len())?;
        stdin.write_all(&body)?;
        stdin.flush()?;
        self.read_frame()
    }

    fn read_frame(&mut self) -> Result<Value, Box<dyn std::error::Error>> {
        loop {
            if let Some(header_end) = self
                .buffer
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
            {
                let header = std::str::from_utf8(&self.buffer[..header_end])?;
                let length = header
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                    .ok_or("missing Content-Length")?;
                let body_start = header_end + 4;
                let body_end = body_start + length;
                if self.buffer.len() >= body_end {
                    let body = self.buffer[body_start..body_end].to_vec();
                    self.buffer.drain(..body_end);
                    return Ok(serde_json::from_slice(&body)?);
                }
            }
            let mut chunk = [0_u8; 4096];
            let stdout = self.child.stdout.as_mut().ok_or("missing child stdout")?;
            let read = stdout.read(&mut chunk)?;
            if read == 0 {
                let mut stderr = String::new();
                if let Some(output) = self.child.stderr.as_mut() {
                    output.read_to_string(&mut stderr)?;
                }
                return Err(format!("MCP process exited before frame: {stderr}").into());
            }
            self.buffer.extend_from_slice(&chunk[..read]);
        }
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        drop(self.child.stdin.take());
        let _ = self.child.wait();
    }
}

#[test]
fn codegraph_stdio_indexes_searches_and_bounds_missing_neighbors()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("dep.js"), "export const value = 1;\n")?;
    std::fs::write(
        root.path().join("entry.js"),
        "import { value } from \"./dep.js\";\nexport const entry = value;\n",
    )?;

    let mut client = McpClient::spawn(env!("CARGO_BIN_EXE_codexy-mcp-codegraph"))?;
    let init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-codegraph");
    let list = client.send(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}))?;
    assert!(
        list["result"]["tools"]
            .as_array()
            .ok_or("tools must be array")?
            .iter()
            .any(|tool| tool["name"] == "codegraph_index")
    );
    let index = client.send(&json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"codegraph_index","arguments":{"root":root.path(),"limit":10}}
    }))?;
    let graph: Value = serde_json::from_str(
        index["result"]["content"][0]["text"]
            .as_str()
            .ok_or("text")?,
    )?;
    assert!(
        graph["edges"]
            .as_array()
            .ok_or("edges must be array")?
            .iter()
            .any(|edge| edge["from"] == "entry.js" && edge["to"] == "dep.js")
    );
    let search = client.send(&json!({
        "jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"codegraph_search","arguments":{"root":root.path(),"query":"value","limit":1}}
    }))?;
    assert!(
        search["result"]["content"][0]["text"]
            .as_str()
            .ok_or("search text")?
            .contains("entry.js:1:")
    );
    let missing = client.send(&json!({
        "jsonrpc":"2.0","id":5,"method":"tools/call",
        "params":{"name":"codegraph_neighbors","arguments":{"root":root.path(),"path":"missing.js"}}
    }))?;
    let neighbors: Value = serde_json::from_str(
        missing["result"]["content"][0]["text"]
            .as_str()
            .ok_or("text")?,
    )?;
    assert_eq!(neighbors, json!([]));
    Ok(())
}

#[test]
fn lsp_stdio_reports_status_diagnostics_and_unmatched_extensions()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let source = root.path().join("sample.js");
    std::fs::write(&source, "const value = 1;\nvalue;\n")?;
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .ok_or("repo root")?;
    let fake_lsp = repo_root.join("tests/mcp/fixtures/lsp/fake-lsp-server.js");

    let mut client = Command::new(env!("CARGO_BIN_EXE_codexy-mcp-lsp"))
        .env("CODEXY_LSP_ALLOW_COMMAND_OVERRIDE", "1")
        .env("CODEXY_FAKE_LSP_PULL_DIAGNOSTICS", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map(|child| McpClient {
            child,
            buffer: Vec::new(),
        })?;
    let init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-lsp");
    let server = json!({"id":"typescript-language-server","command":["node", fake_lsp]});
    let status = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"lsp_status","arguments":{"root":root.path(),"path":"sample.js","server":server}}
    }))?;
    let status_payload: Value = serde_json::from_str(
        status["result"]["content"][0]["text"]
            .as_str()
            .ok_or("text")?,
    )?;
    assert_eq!(status_payload["available"], true);
    let diagnostics = client.send(&json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"lsp_diagnostics","arguments":{"root":root.path(),"path":"sample.js","server":server,"timeoutMs":5000}}
    }))?;
    let diagnostics_payload: Value = serde_json::from_str(
        diagnostics["result"]["content"][0]["text"]
            .as_str()
            .ok_or("text")?,
    )?;
    assert_eq!(diagnostics_payload["status"], "ok");
    let unmatched = client.send(&json!({
        "jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"lsp_status","arguments":{"root":root.path(),"path":"sample.unknown"}}
    }))?;
    let unmatched_payload: Value = serde_json::from_str(
        unmatched["result"]["content"][0]["text"]
            .as_str()
            .ok_or("text")?,
    )?;
    assert_eq!(unmatched_payload["available"], false);
    assert!(
        unmatched_payload["reason"]
            .as_str()
            .ok_or("reason")?
            .contains("no LSP server matches")
    );
    Ok(())
}
