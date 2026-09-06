use std::fs;

use serde_json::{Value, json};

use super::McpClient;

#[cfg(unix)]
#[test]
fn codegraph_stdio_refreshes_imports_when_a_symlinked_go_module_changes()
-> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::symlink;

    let repository = tempfile::tempdir()?;
    let module_target = tempfile::tempdir()?;
    fs::write(
        module_target.path().join("go.mod"),
        "module example.com/old/app\n",
    )?;
    symlink(
        module_target.path().join("go.mod"),
        repository.path().join("go.mod"),
    )?;
    fs::create_dir(repository.path().join("pkg"))?;
    fs::write(
        repository.path().join("main.go"),
        "package main\nimport \"example.com/old/app/pkg\"\nfunc main() { pkg.Run() }\n",
    )?;
    fs::write(
        repository.path().join("pkg/pkg.go"),
        "package pkg\nfunc Run() {}\n",
    )?;

    let mut client = McpClient::spawn_in(
        env!("CARGO_BIN_EXE_codexy-mcp-codegraph"),
        repository.path(),
    )?;
    let first = client.send(&json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"codegraph_index","arguments":{"root":repository.path()}}
    }))?;
    let first_graph: Value = serde_json::from_str(
        first["result"]["content"][0]["text"]
            .as_str()
            .ok_or("first graph text")?,
    )?;
    assert!(first_graph["edges"].as_array().is_some_and(|edges| {
        edges.iter().any(|edge| {
            edge["from"] == "main.go" && edge["to"] == "pkg/pkg.go" && edge["resolved"] == true
        })
    }));

    fs::write(
        module_target.path().join("go.mod"),
        "module example.com/new/app\n",
    )?;
    let second = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"codegraph_index","arguments":{"root":repository.path()}}
    }))?;
    let second_graph: Value = serde_json::from_str(
        second["result"]["content"][0]["text"]
            .as_str()
            .ok_or("second graph text")?,
    )?;
    assert!(!second_graph["edges"].as_array().is_some_and(|edges| {
        edges.iter().any(|edge| {
            edge["from"] == "main.go" && edge["to"] == "pkg/pkg.go" && edge["resolved"] == true
        })
    }));
    Ok(())
}

#[cfg(not(unix))]
#[test]
fn codegraph_stdio_symlinked_go_module_fixture_is_unavailable_on_non_unix() {
    eprintln!("symlinked go.mod fixture is unavailable on this operating system");
}
