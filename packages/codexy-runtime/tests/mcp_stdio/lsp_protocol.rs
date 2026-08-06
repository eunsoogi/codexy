use super::*;
use super::file_uri::decode_local_file_uri;

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
    assert!(
        unmatched_payload["reason"]
            .as_str()
            .ok_or("reason")?
            .contains("no LSP server matches")
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

#[test]
fn lsp_stdio_separates_repository_path_resolution_from_workspace_root_and_rejects_workspace_errors()
-> Result<(), Box<dyn std::error::Error>> {
    let repository = tempfile::tempdir()?;
    let workspace = repository.path().join("packages/codexy-runtime");
    let source = workspace.join("src/sample.rs");
    let capture = repository.path().join("capture.json");
    std::fs::create_dir_all(source.parent().ok_or("source parent")?)?;
    std::fs::write(workspace.join("Cargo.toml"), "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n")?;
    std::fs::write(&source, "fn fixture() {}\n")?;
    let fake_lsp = env!("CARGO_BIN_EXE_codexy-fake-lsp");
    let mut client = Command::new(env!("CARGO_BIN_EXE_codexy-mcp-lsp"))
        .env("CODEXY_LSP_ALLOW_COMMAND_OVERRIDE", "1")
        .env("CODEXY_FAKE_LSP_CAPTURE", &capture)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map(|child| McpClient { child, buffer: Vec::new() })?;
    let _init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    let server = json!({"id":"rust-analyzer","command":[fake_lsp]});
    let arguments = json!({
        "root": repository.path(),
        "workspaceRoot": workspace,
        "path": "packages/codexy-runtime/src/sample.rs",
        "server": server,
        "timeoutMs": 5000
    });
    let response = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"lsp_diagnostics","arguments":arguments}
    }))?;
    let payload: Value = serde_json::from_str(
        response["result"]["content"][0]["text"].as_str().ok_or("diagnostics text")?,
    )?;
    assert_eq!(payload["status"], "ok");
    let captured: Value = serde_json::from_str(&std::fs::read_to_string(&capture)?)?;
    let canonical_workspace = workspace.canonicalize()?;
    assert!(same_path_identity(
        Path::new(captured["cwd"].as_str().ok_or("captured cwd")?),
        &canonical_workspace,
    ));
    assert!(same_path_identity(
        Path::new(r"D:\codexy\packages\codexy-runtime"),
        Path::new(r"\\?\D:\codexy\packages\codexy-runtime"),
    ));
    assert!(!same_path_identity(
        Path::new(r"D:\codexy\packages\codexy-runtime"),
        Path::new(r"\\?\D:\codexy"),
    ));
    let root_uri = captured["rootUri"].as_str().ok_or("captured root URI")?;
    assert!(same_path_identity(
        &decode_local_file_uri(root_uri)?,
        &canonical_workspace,
    ));
    assert!(same_path_identity(
        &decode_local_file_uri("file:///D:/codexy/runtime%20root")?,
        Path::new(r"D:\codexy\runtime root"),
    ));
    assert!(!same_path_identity(
        &decode_local_file_uri("file:///D:/codexy/runtime%20root")?,
        Path::new(r"D:\codexy"),
    ));
    assert!(decode_local_file_uri("file://D%3A%5Ccodexy").is_err());
    assert!(decode_local_file_uri("file:///D:/codexy%2").is_err());

    let poisoned_capture = repository.path().join("poisoned-capture.json");
    let mut poisoned = Command::new(env!("CARGO_BIN_EXE_codexy-mcp-lsp"))
        .env("CODEXY_LSP_ALLOW_COMMAND_OVERRIDE", "1")
        .env("CODEXY_FAKE_LSP_CAPTURE", poisoned_capture)
        .env("CODEXY_FAKE_LSP_STDERR", "FetchWorkspaceError: failed to find any projects")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map(|child| McpClient { child, buffer: Vec::new() })?;
    let _init = poisoned.send(&json!({"jsonrpc":"2.0","id":3,"method":"initialize","params":{}}))?;
    let response = poisoned.send(&json!({
        "jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"lsp_diagnostics","arguments":{
            "root": repository.path(),
            "workspaceRoot": workspace,
            "path":"packages/codexy-runtime/src/sample.rs",
            "server":{"id":"rust-analyzer","command":[fake_lsp]},
            "timeoutMs":5000
        }}
    }))?;
    let payload: Value = serde_json::from_str(
        response["result"]["content"][0]["text"].as_str().ok_or("poisoned diagnostics text")?,
    )?;
    assert_eq!(payload["status"], "error");
    assert!(payload["reason"].as_str().ok_or("error reason")?.contains("FetchWorkspaceError"));
    Ok(())
}

fn same_path_identity(left: &Path, right: &Path) -> bool {
    canonical_path_text(left) == canonical_path_text(right)
}

fn canonical_path_text(path: &Path) -> String {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let normalized = path.to_string_lossy().replace('\\', "/");
    let normalized = normalized.strip_prefix("//?/").unwrap_or(&normalized);
    let is_windows_path = normalized.as_bytes().get(1) == Some(&b':');
    if is_windows_path {
        normalized.to_ascii_lowercase()
    } else {
        normalized.to_owned()
    }
}
