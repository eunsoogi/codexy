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
fn candidate_keeps_windows_native_until_verified_activation()
-> Result<(), Box<dyn std::error::Error>> {
    let candidate = super::workflow("runtime-candidate.yml")?;
    let matrix = candidate["jobs"]["build-runtime"]["strategy"]["matrix"]["include"]
        .as_sequence()
        .ok_or("candidate build matrix")?;
    assert!(matrix.iter().any(|entry| {
        entry["platform"] == "windows-x86_64" && entry["runner"] == "windows-latest"
    }));
    let steps = candidate["jobs"]["build-runtime"]["steps"]
        .as_sequence()
        .ok_or("candidate build steps")?;
    let (_, native) = super::named_step(steps, "Smoke native Windows MCP protocols")?;
    assert_eq!(native["shell"], "pwsh");
    support::assert_structured_literals(
        native["run"].as_str().ok_or("native Windows smoke")?,
        "native Windows candidate proof",
        &[
            "ProcessStartInfo",
            "codexy-mcp-lsp.exe",
            "codexy-mcp-codegraph.exe",
            "tools/call",
        ],
    );
    let assembly = super::run(
        &candidate,
        "stage-runtime",
        "Assemble canonical staged archive and receipt",
    )?;
    assert_eq!(assembly, "scripts/assemble-runtime-candidate");
    let assembly = super::script("assemble-runtime-candidate")?;
    support::assert_structured_literals(
        &assembly,
        "candidate-only Windows activation staging",
        &[
            "windows-x86_64",
            "extension = \"exe\" if platform == \"windows-x86_64\" else \"bin\"",
            "manifest[\"supportedPlatforms\"] = [\"darwin-arm64\", \"linux-x86_64\", \"windows-x86_64\"]",
            "codexy-mcp-devtools-windows-x86_64.exe",
            "codexy-mcp-devtools.exe",
        ],
    );

    let selected = super::workflow("plugin-runtime-binaries.yml")?;
    let windows = super::run(
        &selected,
        "verify-windows-selected-candidate",
        "Verify immutable native Windows candidate bytes",
    )?;
    support::assert_structured_literals(
        windows,
        "selected Windows runtime truth boundary",
        &[
            "legacy-public baseline intentionally has no selected Windows candidate",
            "candidate-proven",
            "tar.exe",
            "codexy-mcp-$server-windows-x86_64.exe",
        ],
    );
    Ok(())
}

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
