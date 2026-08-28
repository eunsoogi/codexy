use crate::support::FixtureCommand as Command;

use tempfile::tempdir;

#[test]
fn rejects_non_json_stdout_and_keeps_valid_json_responses() {
    let root = tempdir().expect("tempdir");
    let checker =
        codexy_runtime::paths::repository_root().join("scripts/inspect-mcp-response");
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
fn parser_matrix_is_cargo_covered_in_one_python_process() {
    let module = codexy_runtime::paths::repository_root()
        .join("scripts/inspect_mcp_response.py");
    let output = Command::new("python3")
        .args([module.to_str().unwrap(), "--matrix"])
        .output()
        .expect("parser matrix");
    assert!(
        output.status.success(),
        "parser matrix failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn workflow_delegates_mcp_stdout_validation_to_the_shared_checker() {
    let root = codexy_runtime::paths::repository_root();
    let workflow: serde_yaml::Value = serde_yaml::from_str(&std::fs::read_to_string(root.join(".github/workflows/plugin-runtime-binaries.yml")).expect("runtime workflow")).expect("workflow YAML");
    let run = workflow["jobs"]["verify-selected-package"]["steps"].as_sequence().and_then(|steps| steps.iter().find(|step| step["name"] == "Assemble state-aware marketplace package without rebuilding")).and_then(|step| step["run"].as_str()).expect("archive inspection step");
    assert!(run.contains("scripts/inspect-release-archive dist/codexy-marketplace-plugin.tar.gz"));
    assert!(run.contains("$staged"));
    let archive = std::fs::read_to_string(root.join("scripts/inspect-release-archive")).expect("archive inspector");
    assert!(archive.lines().any(|line| {
        line.contains("response_checker=") && line.contains("inspect-mcp-response")
    }));
    assert!(archive.lines().any(|line| {
        line.contains("$response_checker")
            && line.contains("$response_file")
            && line.contains("$server")
    }));
}
