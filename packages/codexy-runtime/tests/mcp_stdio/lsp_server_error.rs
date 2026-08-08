use super::*;

#[test]
fn lsp_stdio_preserves_server_json_errors_after_shutdown()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let source = root.path().join("sample.toml");
    std::fs::write(&source, "value = 1\n")?;
    let fake_lsp = env!("CARGO_BIN_EXE_codexy-fake-lsp");
    let mut client = Command::new(env!("CARGO_BIN_EXE_codexy-mcp-lsp"))
        .env("CODEXY_LSP_ALLOW_COMMAND_OVERRIDE", "1")
        .env("CODEXY_FAKE_LSP_RESPONSE_ERROR", "fixture server failure")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map(|child| McpClient { child, buffer: Vec::new() })?;
    let _init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    let response = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"lsp_diagnostics","arguments":{
            "root":root.path(),"path":"sample.toml",
            "server":{"id":"taplo","command":[fake_lsp]},"timeoutMs":5000
        }}
    }))?;
    let payload: Value = serde_json::from_str(
        response["result"]["content"][0]["text"].as_str().ok_or("error payload")?,
    )?;
    assert_eq!(payload["status"], "error");
    assert_eq!(payload["error"]["message"], "fixture server failure");
    Ok(())
}
