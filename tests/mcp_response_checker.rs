use std::process::Command;

use tempfile::tempdir;

#[test]
fn rejects_boolean_wrong_version_and_duplicate_ids() {
    let root = tempdir().expect("tempdir");
    let checker =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/inspect-mcp-response");
    let cases = [
        (
            "{\"jsonrpc\":\"2.0\",\"id\":true,\"result\":{}}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{}}\n",
            "boolean",
        ),
        (
            "{\"jsonrpc\":\"1.0\",\"id\":1,\"result\":{}}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{}}\n",
            "version",
        ),
        (
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{}}\n",
            "duplicate",
        ),
    ];
    for (index, (payload, name)) in cases.into_iter().enumerate() {
        let file = root.path().join(format!("{index}.jsonl"));
        std::fs::write(&file, payload).expect("response fixture");
        let output = Command::new(&checker)
            .args([file.to_str().unwrap(), "lsp"])
            .output()
            .expect("checker");
        assert!(!output.status.success(), "{name} response should fail");
    }
}

#[test]
fn rejects_non_json_stdout_and_keeps_valid_json_responses() {
    let root = tempdir().expect("tempdir");
    let checker =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/inspect-mcp-response");
    let valid = root.path().join("valid.jsonl");
    std::fs::write(
        &valid,
        "\n  \t\n{\"jsonrpc\":\"2.0\",\"method\":\"notifications/message\",\"params\":{}}\n{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"serverInfo\":{\"name\":\"codexy-lsp\"}}}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"lsp_status\"}]}}\n",
    )
    .expect("valid response fixture");
    assert!(
        Command::new(&checker)
            .args([valid.to_str().unwrap(), "lsp"])
            .status()
            .expect("checker")
            .success()
    );

    let contaminated = root.path().join("contaminated.jsonl");
    std::fs::write(
        &contaminated,
        "runtime banner\n{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{}}\n",
    )
    .expect("contaminated response fixture");
    let contaminated_output = Command::new(&checker)
        .args([contaminated.to_str().unwrap(), "lsp"])
        .output()
        .expect("checker");
    assert!(!contaminated_output.status.success());
    assert!(String::from_utf8_lossy(&contaminated_output.stdout).is_empty());
    let stderr = String::from_utf8_lossy(&contaminated_output.stderr);
    assert!(stderr.contains("non-JSON MCP stdout"));
    assert!(!stderr.contains("runtime banner"));
}

#[test]
fn rejects_structured_stdout_noise() {
    let root = tempdir().expect("tempdir");
    let checker =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/inspect-mcp-response");
    let response = root.path().join("structured-noise.jsonl");
    std::fs::write(
        &response,
        "{\"level\":\"info\"}\n{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"serverInfo\":{\"name\":\"codexy-lsp\"}}}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"lsp_status\"}]}}\n",
    )
    .expect("response fixture");
    let output = Command::new(&checker)
        .args([response.to_str().unwrap(), "lsp"])
        .output()
        .expect("checker");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("id-less MCP stdout"));
}

#[test]
fn rejects_non_object_json_stdout() {
    let root = tempdir().expect("tempdir");
    let checker =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/inspect-mcp-response");
    for (name, noise) in [("array", "[]"), ("string", "\"info\""), ("number", "1")] {
        let response = root.path().join(format!("{name}.jsonl"));
        std::fs::write(
            &response,
            format!(
                "{noise}\n{{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{{\"serverInfo\":{{\"name\":\"codexy-lsp\"}}}}}}\n{{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{{\"tools\":[{{\"name\":\"lsp_status\"}}]}}}}\n"
            ),
        )
        .expect("response fixture");
        let output = Command::new(&checker)
            .args([response.to_str().unwrap(), "lsp"])
            .output()
            .expect("checker");
        assert!(!output.status.success(), "{name} stdout should fail");
        assert!(String::from_utf8_lossy(&output.stderr).contains("non-object MCP stdout"));
    }
}

#[test]
fn rejects_wrong_mcp_server_and_tool_identity() {
    let root = tempdir().expect("tempdir");
    let checker =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/inspect-mcp-response");
    let cases = [
        (
            "wrong-server",
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"serverInfo\":{\"name\":\"codexy-codegraph\"}}}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"codegraph_index\"}]}}\n",
            "unexpected MCP server identity",
        ),
        (
            "wrong-tool",
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"serverInfo\":{\"name\":\"codexy-lsp\"}}}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"codegraph_index\"}]}}\n",
            "expected MCP tool is missing",
        ),
    ];
    for (name, payload, diagnostic) in cases {
        let response = root.path().join(format!("{name}.jsonl"));
        std::fs::write(&response, payload).expect("response fixture");
        let output = Command::new(&checker)
            .args([response.to_str().unwrap(), "lsp"])
            .output()
            .expect("checker");
        assert!(!output.status.success(), "{name} identity should fail");
        assert!(String::from_utf8_lossy(&output.stderr).contains(diagnostic));
    }
}

#[test]
fn rejects_unsolicited_json_rpc_response_ids() {
    let root = tempdir().expect("tempdir");
    let checker =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/inspect-mcp-response");
    let response = root.path().join("unsolicited-id.jsonl");
    std::fs::write(
        &response,
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{}}\n{\"jsonrpc\":\"2.0\",\"id\":99,\"result\":{}}\n",
    )
    .expect("response fixture");
    let output = Command::new(&checker)
        .args([response.to_str().unwrap(), "lsp"])
        .output()
        .expect("checker");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unexpected MCP response id"));
}

#[test]
fn workflow_delegates_mcp_stdout_validation_to_the_shared_checker() {
    let workflow = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(".github/workflows/plugin-runtime-binaries.yml"),
    )
    .expect("runtime workflow");
    assert!(workflow.contains("scripts/inspect-mcp-response \"$response_file\" \"$server\""));
    assert!(!workflow.contains("except json.JSONDecodeError: continue"));
    assert!(workflow.contains("capture_output=True"));
    assert!(workflow.contains("write(completed.stdout)"));
    assert!(!workflow.contains("write(completed.stderr)"));
}
