use std::{path::{Path, PathBuf}, process::Command};

use serde_json::{Value, json};

use crate::support::TestResult;

#[test]
fn resolver_preserves_named_specialist_first_luna_default_and_fail_closed_routes() -> TestResult {
    let root = root();
    assert_route(
        &root,
        json!({"schema":"codexy.child-routing-request.v1","classification":"general","codex_thread_operation":"create_thread","codex_thread_capabilities":{"models":[{"model":"gpt-5.6-terra","thinking":["high"]}]}}),
        json!({"route":"generic","codex_thread_operation":"create_thread","model":"gpt-5.6-terra","thinking":"high"}),
    )?;
    assert_route(
        &root,
        json!({"schema":"codexy.child-routing-request.v1","classification":"simple","simple_predicates":{"fixed_scope":true,"deterministic_oracle":true,"low_risk_reversible":true,"no_unresolved_decision":true},"codex_thread_operation":"create_thread","codex_thread_capabilities":{"models":[{"model":"gpt-5.6-luna","thinking":["max"]}]}}),
        json!({"route":"generic","codex_thread_operation":"create_thread","model":"gpt-5.6-luna","thinking":"max"}),
    )?;
    assert_route(
        &root,
        json!({"schema":"codexy.child-routing-request.v1","classification":"general","named_specialist":"codexy-architect","codex_thread_operation":"create_thread"}),
        json!({"route":"named_specialist","agent_type":"codexy-architect"}),
    )?;
    for classification in ["ambiguous", "high_risk", "incomplete"] {
        assert_route(
            &root,
            json!({"schema":"codexy.child-routing-request.v1","classification":classification,"codex_thread_operation":"create_thread"}),
            json!({"route":"root_or_named_specialist"}),
        )?;
    }
    let unknown = resolve(
        &root,
        json!({"schema":"codexy.child-routing-request.v1","classification":"general","named_specialist":"codexy-unknown","codex_thread_operation":"create_thread"}),
    )?;
    assert!(!unknown.status.success(), "unknown named specialist passed");
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
    assert_route(
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
    assert_route(
        &root,
        json!({
            "schema":"codexy.child-routing-request.v1",
            "classification":"general",
            "codex_thread_operation":"create_thread",
            "codex_thread_capabilities":{"models":[]}
        }),
        json!({"route":"root_or_named_specialist"}),
    )?;
    assert_route(
        &root,
        json!({
            "schema":"codexy.child-routing-request.v1",
            "classification":"general",
            "codex_thread_operation":"send_message_to_thread",
            "codex_thread_direction":"child_to_root",
            "codex_thread_capabilities":{"models":[{"model":"gpt-5.6-sol","thinking":["medium"]}]}
        }),
        json!({"route":"child_to_root","codex_thread_operation":"send_message_to_thread","model":"gpt-5.6-sol","thinking":"medium"}),
    )?;
    assert_route(
        &root,
        json!({
            "schema":"codexy.child-routing-request.v1",
            "classification":"general",
            "codex_thread_operation":"send_message_to_thread",
            "codex_thread_direction":"parent_to_generic",
            "codex_thread_capabilities":{"models":[
                {"model":"gpt-5.6-sol","thinking":["medium"]},
                {"model":"gpt-5.6-luna","thinking":["max"]}
            ]}
        }),
        json!({"route":"parent_to_generic","codex_thread_operation":"send_message_to_thread","model":"gpt-5.6-luna","thinking":"max"}),
    )?;
    assert_route(
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
    let invalid = resolve(
        &root,
        json!({
            "schema":"codexy.child-routing-request.v1",
            "classification":"general",
            "codex_thread_operation":"unsupported_operation"
        }),
    )?;
    assert!(!invalid.status.success(), "unsupported operation passed");
    Ok(())
}

fn root() -> PathBuf {
    codexy_runtime::paths::repository_root().join("plugins/codexy")
}

fn assert_route(root: &Path, request: Value, expected: Value) -> TestResult {
    let output = resolve(root, request)?;
    assert!(
        output.status.success(),
        "resolver failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(serde_json::from_slice::<Value>(&output.stdout)?, expected);
    Ok(())
}

fn resolve(root: &Path, request: Value) -> TestResult<std::process::Output> {
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
