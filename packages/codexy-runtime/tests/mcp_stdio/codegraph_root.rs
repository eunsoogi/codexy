//! Confirms malformed explicit root values fail as JSON-RPC tool errors rather
//! than being coerced into a default repository path.

use super::*;

#[cfg(unix)]
#[test]
fn codegraph_stdio_honors_absolute_root_after_startup_cwd_is_removed()
-> Result<(), Box<dyn std::error::Error>> {
    let repository = tempfile::tempdir()?;
    std::fs::write(
        repository.path().join("entry.rs"),
        "pub const EXPLICIT_ROOT_MARKER: u8 = 1;\n",
    )?;

    // A live stdio process can outlast the directory from which its host launched it.
    let startup_cwd = tempfile::tempdir()?;
    let startup_cwd_path = startup_cwd.path().to_path_buf();
    let mut client = McpClient::spawn_in(
        env!("CARGO_BIN_EXE_codexy-mcp-codegraph"),
        &startup_cwd_path,
    )?;
    std::fs::remove_dir(&startup_cwd_path)?;

    let response = client.send(&json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"codegraph_search","arguments":{
            "root":repository.path(),"query":"EXPLICIT_ROOT_MARKER","limit":10
        }}
    }))?;
    let error = response["error"]["message"]
        .as_str()
        .unwrap_or("no JSON-RPC error message");
    assert!(response.get("result").is_some(), "MCP call failed: {error}");

    let text = response["result"]["content"][0]["text"]
        .as_str()
        .ok_or("search result text")?;
    let output: Value = serde_json::from_str(text)?;
    assert!(
        output["matches"].as_array().ok_or("search matches")?.iter().any(|item| {
            item.as_str()
                == Some("./entry.rs:1:pub const EXPLICIT_ROOT_MARKER: u8 = 1;")
        }),
        "the explicit repository root should be searched after the process CWD is removed"
    );

    // Default and relative roots still depend on the process CWD and keep their existing error.
    for (id, arguments) in [
        (
            2,
            json!({"root":".","query":"EXPLICIT_ROOT_MARKER","limit":10}),
        ),
        (3, json!({"query":"EXPLICIT_ROOT_MARKER","limit":10})),
    ] {
        let response = client.send(&json!({
            "jsonrpc":"2.0","id":id,"method":"tools/call",
            "params":{"name":"codegraph_search","arguments":arguments}
        }))?;
        assert_eq!(response["error"]["code"], -32000);
        let message = response["error"]["message"]
            .as_str()
            .ok_or("missing CWD error message")?;
        assert!(
            message.contains("codegraph root_unreadable at .: unable to determine current directory:"),
            "unexpected CWD error: {message}"
        );
        assert!(response.get("result").is_none());
    }
    Ok(())
}

#[test]
fn codegraph_stdio_rejects_explicit_non_string_roots() -> Result<(), Box<dyn std::error::Error>> {
    let repository = tempfile::tempdir()?;
    let mut client = McpClient::spawn_in(
        env!("CARGO_BIN_EXE_codexy-mcp-codegraph"),
        repository.path(),
    )?;
    for (id, root) in [(1, Value::Null), (2, json!(42))] {
        let response = client.send(&json!({
            "jsonrpc":"2.0","id":id,"method":"tools/call",
            "params":{"name":"codegraph_index","arguments":{"root":root}}
        }))?;
        assert_eq!(response["error"]["code"], -32000);
        let message = response["error"]["message"]
            .as_str()
            .ok_or("missing invalid-root error message")?;
        assert!(message.contains("root_invalid"), "unexpected error: {message}");
        assert!(response.get("result").is_none());
    }
    Ok(())
}
