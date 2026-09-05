use super::*;

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
