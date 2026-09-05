use super::*;

#[path = "lsp_batch/validation.rs"]
mod validation;
#[path = "lsp_batch/installed.rs"]
mod installed;

fn start_client(
    extra_env: &[(&str, &str)],
    capture: Option<&Path>,
) -> Result<McpClient, Box<dyn std::error::Error>> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_codexy-mcp-lsp"));
    command.env("CODEXY_LSP_ALLOW_COMMAND_OVERRIDE", "1");
    if let Some(capture) = capture {
        command.env("CODEXY_FAKE_LSP_CAPTURE", capture);
    }
    for (name, value) in extra_env {
        command.env(name, value);
    }
    let child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    Ok(McpClient {
        child,
        buffer: Vec::new(),
    })
}

fn initialize(client: &mut McpClient) -> Result<Value, Box<dyn std::error::Error>> {
    client.send(&json!({
        "jsonrpc":"2.0","id":1,"method":"initialize","params":{}
    }))
}

fn batch_response(
    client: &mut McpClient,
    id: u64,
    arguments: Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    client.send(&json!({
        "jsonrpc":"2.0","id":id,"method":"tools/call",
        "params":{"name":"lsp_batch","arguments":arguments}
    }))
}

fn batch_arguments(root: &Path, fake_lsp: &str, requests: Value) -> Value {
    json!({
        "root":root,
        "workspaceRoot":root,
        "server":{"id":"taplo","command":[fake_lsp]},
        "requests":requests
    })
}

fn text_payload(response: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(
        response["result"]["content"][0]["text"]
            .as_str()
            .ok_or("batch text")?,
    )?)
}

#[test]
fn lsp_batch_is_listed_with_bounded_schema() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = start_client(&[], None)?;
    initialize(&mut client)?;
    let response = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/list","params":{}
    }))?;
    let tool = response["result"]["tools"]
        .as_array()
        .ok_or("tools")?
        .iter()
        .find(|tool| tool["name"] == "lsp_batch")
        .ok_or("lsp_batch tool")?;
    assert_eq!(tool["inputSchema"]["properties"]["requests"]["maxItems"], 8);
    assert_eq!(
        tool["inputSchema"]["properties"]["requests"]["items"]["properties"]["method"]["enum"],
        json!(["lsp_document_symbols", "lsp_definition", "lsp_references", "lsp_diagnostics"])
    );
    Ok(())
}

#[test]
fn lsp_batch_initializes_once_and_returns_ordered_results() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("first.toml"), "value = 1\n")?;
    std::fs::write(root.path().join("second.toml"), "value = 2\n")?;
    let fake_lsp = env!("CARGO_BIN_EXE_codexy-fake-lsp");
    let capture = root.path().join("capture.json");
    let mut client = start_client(&[], Some(&capture))?;
    initialize(&mut client)?;
    let response = batch_response(
        &mut client,
        2,
        batch_arguments(
            root.path(),
            fake_lsp,
            json!([
                {"method":"lsp_document_symbols","path":"first.toml"},
                {"method":"lsp_document_symbols","path":"second.toml"}
            ]),
        ),
    )?;
    let payload = text_payload(&response)?;
    assert_eq!(payload["status"], "ok");
    assert_eq!(payload["results"].as_array().ok_or("results")?.len(), 2);
    let capture_payload: Value = serde_json::from_str(&std::fs::read_to_string(capture)?)?;
    assert_eq!(capture_payload["initializeCount"], 1);
    assert_eq!(capture_payload["requestCount"], 2);
    assert_eq!(capture_payload["requestIds"], json!([2, 3]));
    Ok(())
}

#[test]
fn lsp_batch_keeps_later_requests_after_one_server_error() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("first.toml"), "value = 1\n")?;
    std::fs::write(root.path().join("second.toml"), "value = 2\n")?;
    let fake_lsp = env!("CARGO_BIN_EXE_codexy-fake-lsp");
    let capture = root.path().join("capture.json");
    let mut client = start_client(
        &[("CODEXY_FAKE_LSP_RESPONSE_ERROR", "one request failed"),
            ("CODEXY_FAKE_LSP_RESPONSE_ERROR_ON_REQUEST", "1")],
        Some(&capture),
    )?;
    initialize(&mut client)?;
    let payload = text_payload(&batch_response(
        &mut client,
        2,
        batch_arguments(
            root.path(),
            fake_lsp,
            json!([
                {"method":"lsp_document_symbols","path":"first.toml"},
                {"method":"lsp_document_symbols","path":"second.toml"}
            ]),
        ),
    )?)?;
    assert_eq!(payload["status"], "error");
    assert_eq!(payload["results"][0]["status"], "error");
    assert_eq!(payload["results"][1]["status"], "ok");
    let capture_payload: Value = serde_json::from_str(&std::fs::read_to_string(capture)?)?;
    assert_eq!(capture_payload["initializeCount"], 1);
    assert_eq!(capture_payload["requestCount"], 2);
    Ok(())
}

#[test]
fn lsp_batch_reports_remaining_requests_after_server_crash() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    for name in ["first.toml", "second.toml", "third.toml"] {
        std::fs::write(root.path().join(name), "value = 1\n")?;
    }
    let fake_lsp = env!("CARGO_BIN_EXE_codexy-fake-lsp");
    let mut client = start_client(&[("CODEXY_FAKE_LSP_CRASH_AFTER_REQUEST", "1")], None)?;
    initialize(&mut client)?;
    let payload = text_payload(&batch_response(
        &mut client,
        2,
        batch_arguments(
            root.path(),
            fake_lsp,
            json!([
                {"method":"lsp_document_symbols","path":"first.toml"},
                {"method":"lsp_document_symbols","path":"second.toml"},
                {"method":"lsp_document_symbols","path":"third.toml"}
            ]),
        ),
    )?)?;
    assert_eq!(payload["status"], "error");
    assert_eq!(payload["results"].as_array().ok_or("results")?.len(), 3);
    assert!(payload["results"][1]["reason"]
        .as_str()
        .ok_or("remaining reason")?
        .contains("not executed"));
    assert!(payload["results"][2]["reason"]
        .as_str()
        .ok_or("remaining reason")?
        .contains("not executed"));
    Ok(())
}

#[test]
fn lsp_batch_caps_request_waits_at_the_batch_deadline() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("first.toml"), "value = 1\n")?;
    std::fs::write(root.path().join("second.toml"), "value = 2\n")?;
    let fake_lsp = env!("CARGO_BIN_EXE_codexy-fake-lsp");
    let capture = root.path().join("capture.json");
    let mut client = start_client(&[("CODEXY_FAKE_LSP_DELAY_MS", "250")], Some(&capture))?;
    initialize(&mut client)?;
    let mut arguments = batch_arguments(
        root.path(),
        fake_lsp,
        json!([
            {"method":"lsp_document_symbols","path":"first.toml"},
            {"method":"lsp_document_symbols","path":"second.toml"}
        ]),
    );
    arguments["timeoutMs"] = json!(100);
    arguments["deadlineMs"] = json!(1000);
    let payload = text_payload(&batch_response(&mut client, 2, arguments)?)?;
    assert_eq!(payload["status"], "error");
    assert!(payload["results"][0]["reason"]
        .as_str()
        .ok_or("timeout reason")?
        .contains("Timed out"));
    assert!(payload["results"][1]["reason"]
        .as_str()
        .ok_or("remaining reason")?
        .contains("not executed"));
    let capture_payload: Value = serde_json::from_str(&std::fs::read_to_string(capture)?)?;
    assert_eq!(capture_payload["shutdownCount"], 1);
    Ok(())
}
