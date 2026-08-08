use super::*;

#[test]
fn lsp_stdio_rejects_an_early_workspace_error_after_the_display_cap()
-> Result<(), Box<dyn std::error::Error>> {
    let payload = diagnostics_payload(Some("FetchWorkspaceError: fixture workspace failure"), 4097)?;
    assert_eq!(payload["status"], "error");
    let reason = payload["reason"]
        .as_str()
        .ok_or("workspace error reason")?;
    assert!(reason.contains("FetchWorkspaceError"));
    assert!(!reason.contains("fixture workspace failure"));
    assert!(reason.ends_with('x'));
    Ok(())
}

#[test]
fn lsp_stdio_keeps_a_long_unrelated_stderr_tail_nonfatal()
-> Result<(), Box<dyn std::error::Error>> {
    let payload = diagnostics_payload(None, 4097)?;
    assert_eq!(payload["status"], "ok");
    let stderr = payload["stderr"].as_str().ok_or("capped stderr")?;
    assert_eq!(stderr.len(), 4000);
    assert!(!stderr.contains("FetchWorkspaceError"));
    Ok(())
}

#[test]
fn lsp_stdio_rejects_a_workspace_error_inside_the_display_tail()
-> Result<(), Box<dyn std::error::Error>> {
    let payload = diagnostics_payload(Some("FetchWorkspaceError: fixture workspace failure"), 0)?;
    assert_eq!(payload["status"], "error");
    assert!(payload["reason"]
        .as_str()
        .ok_or("workspace error reason")?
        .contains("FetchWorkspaceError"));
    Ok(())
}

fn diagnostics_payload(stderr: Option<&str>, tail: usize) -> Result<Value, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let source = root.path().join("sample.toml");
    std::fs::write(&source, "value = 1\n")?;
    let mut command = Command::new(env!("CARGO_BIN_EXE_codexy-mcp-lsp"));
    command
        .env("CODEXY_LSP_ALLOW_COMMAND_OVERRIDE", "1")
        .env("CODEXY_FAKE_LSP_STDERR_TAIL_BYTES", tail.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(stderr) = stderr {
        command.env("CODEXY_FAKE_LSP_STDERR", stderr);
    }
    let mut client = command.spawn().map(|child| McpClient { child, buffer: Vec::new() })?;
    let _init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    let response = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"lsp_diagnostics","arguments":{
            "root":root.path(),"path":"sample.toml",
            "server":{"id":"taplo","command":[env!("CARGO_BIN_EXE_codexy-fake-lsp")]},"timeoutMs":5000
        }}
    }))?;
    Ok(serde_json::from_str(
        response["result"]["content"][0]["text"].as_str().ok_or("diagnostics payload")?,
    )?)
}
