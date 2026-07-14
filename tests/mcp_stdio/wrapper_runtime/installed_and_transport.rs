use super::super::*;

#[test]
fn lsp_wrapper_uses_installed_plugin_root_for_config() -> Result<(), Box<dyn std::error::Error>> {
    let installed_plugin = installed_plugin_copy()?;
    let lsp_config_path = installed_plugin.path.join(".codex/lsp-client.json");
    let mut lsp_config: Value = serde_json::from_str(&std::fs::read_to_string(&lsp_config_path)?)?;
    lsp_config["lsp"]["codexy-installed-root"] = json!({
        "extensions": [".installed"],
        "priority": 999,
        "command": [env!("CARGO_BIN_EXE_codexy-fake-lsp")]
    });
    std::fs::write(&lsp_config_path, serde_json::to_vec_pretty(&lsp_config)?)?;
    std::fs::write(
        installed_plugin.path.join("sample.installed"),
        "value = 1\n",
    )?;

    let mut command = Command::new(installed_plugin.path.join("mcp/codexy-mcp-lsp"));
    command
        .current_dir(&installed_plugin.path)
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut client = McpClient::spawn_command(command)?;
    let _init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    let response = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"lsp_for_path","arguments":{"path":"sample.installed"}}
    }))?;
    let payload: Value = serde_json::from_str(
        response["result"]["content"][0]["text"]
            .as_str()
            .ok_or("lsp_for_path text")?,
    )?;
    assert!(
        payload
            .as_array()
            .ok_or("lsp_for_path payload must be array")?
            .iter()
            .any(|server| server["id"] == "codexy-installed-root"),
        "wrapper-launched runtime must read LSP config from copied installed plugin, got {payload:#}"
    );
    let status = client.send(&json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"lsp_status","arguments":{"root":&installed_plugin.path,"path":"sample.installed","server":null}}
    }))?;
    let status_payload: Value = serde_json::from_str(
        status["result"]["content"][0]["text"]
            .as_str()
            .ok_or_else(|| format!("lsp_status text missing in {status:#}"))?,
    )?;
    assert_eq!(status_payload["server"]["id"], "codexy-installed-root");

    let diagnostics = client.send(&json!({
        "jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"lsp_diagnostics","arguments":{"root":&installed_plugin.path,"path":"sample.installed","server":null,"timeoutMs":5000.0}}
    }))?;
    let diagnostics_payload: Value = serde_json::from_str(
        diagnostics["result"]["content"][0]["text"]
            .as_str()
            .ok_or_else(|| format!("lsp_diagnostics text missing in {diagnostics:#}"))?,
    )?;
    assert_eq!(
        diagnostics_payload["status"], "ok",
        "server:null diagnostics should use the installed plugin config, got {diagnostics_payload:#}"
    );
    Ok(())
}

#[test]
fn codegraph_wrapper_uses_bundled_runtime_without_global_path()
-> Result<(), Box<dyn std::error::Error>> {
    let installed_plugin = installed_plugin_copy()?;
    let mut command = Command::new(installed_plugin.path.join("mcp/codexy-mcp-codegraph"));
    command
        .current_dir(&installed_plugin.path)
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut client = McpClient::spawn_command(command)?;
    let init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-codegraph");
    Ok(())
}

#[test]
fn codegraph_stdio_accepts_newline_delimited_json_rpc() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = McpClient::spawn(env!("CARGO_BIN_EXE_codexy-mcp-codegraph"))?;
    let init =
        client.send_line(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-codegraph");
    let list =
        client.send_line(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}))?;
    assert!(
        list["result"]["tools"]
            .as_array()
            .ok_or("tools must be array")?
            .iter()
            .any(|tool| tool["name"] == "codegraph_index"),
        "newline stdio tools/list must include codegraph_index, got {list:#}"
    );
    Ok(())
}

#[test]
fn lsp_stdio_accepts_newline_delimited_json_rpc() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = McpClient::spawn(env!("CARGO_BIN_EXE_codexy-mcp-lsp"))?;
    let init =
        client.send_line(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-lsp");
    let list =
        client.send_line(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}))?;
    assert!(
        list["result"]["tools"]
            .as_array()
            .ok_or("tools must be array")?
            .iter()
            .any(|tool| tool["name"] == "lsp_status"),
        "newline stdio tools/list must include lsp_status, got {list:#}"
    );
    Ok(())
}

#[test]
fn content_length_stdio_accepts_leading_content_type_header()
-> Result<(), Box<dyn std::error::Error>> {
    let mut client = McpClient::spawn(env!("CARGO_BIN_EXE_codexy-mcp-codegraph"))?;
    let init = client.send_with_leading_content_type(
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
    )?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-codegraph");
    let list = client.send_with_leading_content_type(
        &json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
    )?;
    assert!(
        list["result"]["tools"]
            .as_array()
            .ok_or("tools must be array")?
            .iter()
            .any(|tool| tool["name"] == "codegraph_index"),
        "leading Content-Type header must not prevent tools/list, got {list:#}"
    );
    Ok(())
}
