use std::{
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::{json, Value};

use crate::support::TestResult;

#[test]
fn resolver_preserves_named_specialist_first_luna_default_and_fail_closed_routes() -> TestResult {
    let root = root();
    assert_in_process_route(
        &root,
        json!({"schema":"codexy.child-routing-request.v1","classification":"general","codex_thread_operation":"create_thread","codex_thread_capabilities":{"models":[{"model":"gpt-5.6-terra","thinking":["high"]}]}}),
        json!({"route":"root_or_named_specialist"}),
    )?;
    assert_in_process_route(
        &root,
        json!({"schema":"codexy.child-routing-request.v1","classification":"simple","simple_predicates":{"fixed_scope":true,"deterministic_oracle":true,"low_risk_reversible":true,"no_unresolved_decision":true},"codex_thread_operation":"create_thread","codex_thread_capabilities":{"models":[{"model":"gpt-5.6-luna","thinking":["max"]}]}}),
        json!({"route":"generic","codex_thread_operation":"create_thread","model":"gpt-5.6-luna","thinking":"max"}),
    )?;
    assert_cli_success(
        &root,
        json!({"schema":"codexy.child-routing-request.v1","classification":"general","named_specialist":"codexy-architect","codex_thread_operation":"create_thread"}),
        json!({"route":"named_specialist","agent_type":"codexy-architect"}),
    )?;
    for classification in ["ambiguous", "high_risk", "incomplete"] {
        assert_in_process_route(
            &root,
            json!({"schema":"codexy.child-routing-request.v1","classification":classification,"codex_thread_operation":"create_thread"}),
            json!({"route":"root_or_named_specialist"}),
        )?;
    }
    assert_in_process_rejects(
        &root,
        json!({"schema":"codexy.child-routing-request.v1","classification":"general","named_specialist":"codexy-unknown","codex_thread_operation":"create_thread"}),
        "child routing request names an unknown packaged specialist",
    )?;
    Ok(())
}

#[test]
fn resolver_preserves_capability_fallback_and_codex_thread_delivery() -> TestResult {
    let root = root();
    let predicates = json!({
        "fixed_scope": true,
        "deterministic_oracle": true,
        "low_risk_reversible": true,
        "no_unresolved_decision": true
    });
    assert_in_process_route(
        &root,
        json!({
            "schema":"codexy.child-routing-request.v1",
            "classification":"simple",
            "simple_predicates":predicates,
            "codex_thread_operation":"create_thread",
            "codex_thread_capabilities":{"models":[
                {"model":"gpt-5.6-terra","thinking":["high"]},
                {"model":"gpt-5.6-luna","thinking":["max"]}
            ]}
        }),
        json!({"route":"generic","codex_thread_operation":"create_thread","model":"gpt-5.6-luna","thinking":"max"}),
    )?;
    assert_in_process_route(
        &root,
        json!({
            "schema":"codexy.child-routing-request.v1",
            "classification":"general",
            "codex_thread_operation":"create_thread",
            "codex_thread_capabilities":{"models":[]}
        }),
        json!({"route":"root_or_named_specialist"}),
    )?;
    assert_in_process_route(
        &root,
        json!({
            "schema":"codexy.child-routing-request.v1",
            "classification":"general",
            "codex_thread_operation":"send_message_to_thread",
            "codex_thread_direction":"child_to_root",
            "codex_thread_capabilities":{"models":[{"model":"gpt-6-astra","thinking":["medium"]}]}
        }),
        json!({"route":"child_to_root","codex_thread_operation":"send_message_to_thread","model":"gpt-6-astra","thinking":"medium"}),
    )?;
    for (model, thinking) in [("gpt-5.6-sol", "medium"), ("gpt-5.6-luna", "max")] {
        assert_in_process_route(
            &root,
            json!({
                "schema":"codexy.child-routing-request.v1",
                "classification":"general",
                "codex_thread_operation":"send_message_to_thread",
                "codex_thread_direction":"child_to_root",
                "codex_thread_capabilities":{"models":[{"model":model,"thinking":[thinking]}]}
            }),
            json!({"route":"root_or_named_specialist"}),
        )?;
    }
    assert_in_process_route(
        &root,
        json!({
            "schema":"codexy.child-routing-request.v1",
            "classification":"general",
            "codex_thread_operation":"send_message_to_thread",
            "codex_thread_direction":"parent_to_generic",
            "codex_thread_capabilities":{"models":[
                {"model":"gpt-6-astra","thinking":["medium"]},
                {"model":"gpt-5.6-luna","thinking":["max"]}
            ]}
        }),
        json!({"route":"parent_to_generic","codex_thread_operation":"send_message_to_thread","model":"gpt-5.6-luna","thinking":"max"}),
    )?;
    assert_in_process_route(
        &root,
        json!({
            "schema":"codexy.child-routing-request.v1",
            "classification":"general",
            "codex_thread_operation":"send_message_to_thread",
            "codex_thread_direction":"parent_to_generic",
            "codex_thread_capabilities":{"models":[{"model":"gpt-5.6-terra","thinking":["high"]}]}
        }),
        json!({"route":"root_or_named_specialist"}),
    )?;
    assert_cli_rejects(
        &root,
        json!({
            "schema":"codexy.child-routing-request.v1",
            "classification":"general",
            "codex_thread_operation":"unsupported_operation"
        }),
        "child routing request names an unsupported Codex thread operation",
    )?;
    Ok(())
}

fn root() -> PathBuf {
    codexy_runtime::paths::repository_root().join("plugins/codexy")
}

fn assert_in_process_route(root: &Path, request: Value, expected: Value) -> TestResult {
    assert_eq!(resolve_in_process(root, &request)?, expected);
    Ok(())
}

fn assert_cli_success(root: &Path, request: Value, expected: Value) -> TestResult {
    let output = resolve_with_cli(root, request)?;
    assert!(
        output.status.success(),
        "resolver failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "successful resolver wrote stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(serde_json::from_slice::<Value>(&output.stdout)?, expected);
    Ok(())
}

fn assert_in_process_rejects(root: &Path, request: Value, expected_error: &str) -> TestResult {
    let error = resolve_in_process(root, &request).expect_err("invalid routing request passed");
    assert!(
        error.to_string().contains(expected_error),
        "unexpected resolver error: {error}"
    );
    Ok(())
}

fn assert_cli_rejects(root: &Path, request: Value, expected_error: &str) -> TestResult {
    let output = resolve_with_cli(root, request)?;
    assert!(!output.status.success(), "invalid routing request passed");
    assert!(
        output.stdout.is_empty(),
        "invalid resolver wrote stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected_error),
        "unexpected resolver error: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn resolve_in_process(root: &Path, request: &Value) -> TestResult<Value> {
    Ok(codexy_runtime::validation::resolve_child_routing(
        root,
        &serde_json::to_string(request)?,
    )?)
}

fn resolve_with_cli(root: &Path, request: Value) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let request_path = temp.path().join("request.json");
    std::fs::write(&request_path, serde_json::to_vec(&request)?)?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            root.to_str().ok_or("plugin root")?,
            "--resolve-child-routing",
            "--routing-request-file",
        ])
        .arg(request_path)
        .output()?)
}
