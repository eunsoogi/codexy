use std::io::{BufRead as _, BufReader, Write as _};
use std::process::{Child, Command, Stdio};

use serde_json::{Value, json};

struct McpClient {
    child: Child,
}

impl McpClient {
    fn spawn() -> Result<Self, Box<dyn std::error::Error>> {
        let child = Command::new(env!("CARGO_BIN_EXE_codexy-mcp-lsp"))
            .env("PATH", "/usr/bin:/bin")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        Ok(Self { child })
    }

    fn send(&mut self, payload: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let body = serde_json::to_vec(payload)?;
        let stdin = self.child.stdin.as_mut().ok_or("missing child stdin")?;
        stdin.write_all(&body)?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
        let stdout = self.child.stdout.as_mut().ok_or("missing child stdout")?;
        let mut line = String::new();
        BufReader::new(stdout).read_line(&mut line)?;
        Ok(serde_json::from_str(&line)?)
    }
}

#[test]
fn lsp_status_classifies_missing_rust_analyzer_as_readiness_defect()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("sample.rs"), "fn main() {}\n")?;

    let mut client = McpClient::spawn()?;
    let init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-lsp");

    let status = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"lsp_status","arguments":{"root":root.path(),"path":"sample.rs"}}
    }))?;
    let status_payload: Value = serde_json::from_str(
        status["result"]["content"][0]["text"]
            .as_str()
            .ok_or("text")?,
    )?;

    assert_eq!(status_payload["server"]["id"], "rust-analyzer");
    assert_eq!(status_payload["available"], false);
    assert_eq!(status_payload["readiness"]["defect"], "missing-executable");
    assert_eq!(
        status_payload["readiness"]["action"],
        "install rust-analyzer or put it on PATH before relying on Rust LSP diagnostics"
    );
    assert!(
        status_payload["reason"]
            .as_str()
            .ok_or("reason")?
            .contains("executable not found on PATH: rust-analyzer")
    );
    Ok(())
}
