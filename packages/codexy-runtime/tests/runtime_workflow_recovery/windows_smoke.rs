use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use serde_json::{Value, json};

use crate::support;

#[path = "../mcp_stdio/client.rs"]
#[allow(dead_code)]
mod client;

use client::McpClient;

#[test]
fn windows_smoke_preserves_nested_watcher_requests() -> Result<(), Box<dyn std::error::Error>> {
    let candidate = super::workflow("runtime-candidate.yml")?;
    let native = super::run(
        &candidate,
        "build-runtime",
        "Smoke native Windows MCP protocols",
    )?;
    support::assert_structured_literals(
        native,
        "nested native Windows watcher request serialization",
        &[
            "parent = @{ id = \"smoke\" }",
            "watcher = @{ id = \"smoke\" }",
            "targets = @(@{ threadId = \"smoke\" })",
            "ConvertTo-Json -Compress -Depth 10",
            "tool smoke failed: $($tool | ConvertTo-Json -Depth 10 -Compress)",
        ],
    );
    assert!(!native.contains("ConvertTo-Json -Compress));"));

    let state = tempfile::tempdir()?;
    let mut command = Command::new(env!("CARGO_BIN_EXE_codexy-mcp-watcher"));
    command.env("CODEXY_WATCHER_STATE_DIR", state.path());
    let mut client = McpClient::spawn_command(command)?;
    let initialize = client.send_line(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }))?;
    assert_eq!(initialize["result"]["protocolVersion"], "2024-11-05");
    let opened = client.send_line(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "watcher_open",
            "arguments": {
                "assignmentId": "runtime-smoke",
                "parent": {"id": "smoke"},
                "watcher": {"id": "smoke"},
                "targets": [{"threadId": "smoke"}],
                "ttlSeconds": 1
            }
        }
    }))?;
    assert!(opened["result"]["content"]
        .as_array()
        .is_some_and(|content| !content.is_empty()));
    Ok(())
}
