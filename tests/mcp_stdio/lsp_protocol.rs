use super::*;

#[test]
fn lsp_stdio_accepts_newline_delimited_json_rpc() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = McpClient::spawn(env!("CARGO_BIN_EXE_codexy-mcp-lsp"))?;
    let init =
        client.send_line(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-lsp");
    let list =
        client.send_line(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}))?;
    assert!(list["result"]["tools"]
        .as_array()
        .ok_or("tools must be array")?
        .iter()
        .any(|tool| tool["name"] == "lsp_status"));
    Ok(())
}

#[test]
fn lsp_stdio_reports_status_diagnostics_and_unmatched_extensions()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let source = root.path().join("sample.toml");
    std::fs::write(&source, "value = 1\n")?;
    let fake_lsp = env!("CARGO_BIN_EXE_codexy-fake-lsp");

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
    let server = json!({"id":"taplo","command":[fake_lsp]});
    let status = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"lsp_status","arguments":{"root":root.path(),"path":"sample.toml","server":server}}
    }))?;
    let status_payload: Value = serde_json::from_str(
        status["result"]["content"][0]["text"]
            .as_str()
            .ok_or("text")?,
    )?;
    assert_eq!(status_payload["available"], true);
    let diagnostics = client.send(&json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"lsp_diagnostics","arguments":{"root":root.path(),"path":"sample.toml","server":server,"timeoutMs":5000}}
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
    // structured-contract: non-contract substring rationale: verifies unsupported-extension diagnostic returned to clients
    assert!(
        unmatched_payload["reason"]
            .as_str()
            .ok_or("reason")?
            .find("no LSP server matches")
            .is_some()
    );
    Ok(())
}

#[test]
fn lsp_stdio_accepts_integer_positions_encoded_as_json_floats()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let source = root.path().join("sample.toml");
    let capture = root.path().join("capture.json");
    std::fs::write(&source, "value = 1\n")?;
    let fake_lsp = env!("CARGO_BIN_EXE_codexy-fake-lsp");

    let mut client = Command::new(env!("CARGO_BIN_EXE_codexy-mcp-lsp"))
        .env("CODEXY_LSP_ALLOW_COMMAND_OVERRIDE", "1")
        .env("CODEXY_FAKE_LSP_CAPTURE", &capture)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map(|child| McpClient {
            child,
            buffer: Vec::new(),
        })?;
    let _init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    let server = json!({"id":"taplo","command":[fake_lsp]});
    let response = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"lsp_definition","arguments":{"root":root.path(),"path":"sample.toml","server":server,"line":1.0,"character":2.0,"timeoutMs":5000.0}}
    }))?;
    let payload: Value = serde_json::from_str(
        response["result"]["content"][0]["text"]
            .as_str()
            .ok_or("definition text")?,
    )?;
    assert_eq!(payload["status"], "ok");
    let capture_payload: Value = serde_json::from_str(&std::fs::read_to_string(capture)?)?;
    assert_eq!(capture_payload["position"]["line"], 1);
    assert_eq!(capture_payload["position"]["character"], 2);
    Ok(())
}
