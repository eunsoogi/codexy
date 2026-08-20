use std::{path::Path, process::Command};

use serde_json::{Value, json};

use crate::support::{self, TestResult};

const POLICY: &str = "skills/orchestration/references/child-routing-policy.json";
const RESULTS: &str = "skills/orchestration/references/routing-evaluation-results.json";

#[test]
fn resolver_uses_closed_policy_for_generic_simple_and_fallback_routes() -> TestResult {
    let fixture = support::plugin_fixture_with_mutable_files(&[Path::new(POLICY), Path::new(RESULTS)])?;
    assert_route(
        fixture.root(),
        json!({"schema":"codexy.child-routing-request.v1","classification":"general","codex_thread_operation":"create_thread","codex_thread_capabilities":{"models":[{"model":"gpt-5.6-terra","thinking":["high"]}]}}),
        json!({"route":"generic","codex_thread_operation":"create_thread","model":"gpt-5.6-terra","thinking":"high"}),
    )?;
    assert_route(
        fixture.root(),
        json!({"schema":"codexy.child-routing-request.v1","classification":"simple","simple_predicates":{"fixed_scope":true,"deterministic_oracle":true,"low_risk_reversible":true,"no_unresolved_decision":true},"codex_thread_operation":"create_thread","codex_thread_capabilities":{"models":[{"model":"gpt-5.6-luna","thinking":["max"]}]}}),
        json!({"route":"generic","codex_thread_operation":"create_thread","model":"gpt-5.6-luna","thinking":"max"}),
    )?;
    assert_route(
        fixture.root(),
        json!({"schema":"codexy.child-routing-request.v1","classification":"general","named_specialist":"codexy-architect","codex_thread_operation":"create_thread"}),
        json!({"route":"named_specialist","agent_type":"codexy-architect"}),
    )?;
    for classification in ["ambiguous", "high_risk", "incomplete"] {
        assert_route(
            fixture.root(),
            json!({"schema":"codexy.child-routing-request.v1","classification":classification,"codex_thread_operation":"create_thread"}),
            json!({"route":"root_or_named_specialist"}),
        )?;
    }
    assert_route(
        fixture.root(),
        json!({
            "schema":"codexy.child-routing-request.v1",
            "classification":"general",
            "codex_thread_operation":"create_thread"
        }),
        json!({"route":"root_or_named_specialist"}),
    )?;
    Ok(())
}

#[test]
fn distributed_general_block_is_measurement_only() -> TestResult {
    let fixture = support::plugin_fixture_with_mutable_files(&[Path::new(POLICY)])?;
    let policy: Value = serde_json::from_str(&std::fs::read_to_string(fixture.root().join(POLICY))?)?;
    let general = policy
        .get("general")
        .and_then(Value::as_object)
        .ok_or("general policy block")?;

    assert!(!general.contains_key("model"), "general block must not expose a route-shaped model");
    assert_eq!(general.get("candidate_efforts"), Some(&json!(["high", "xhigh", "max"])));
    assert_eq!(
        general.get("measurement_results"),
        Some(&json!(RESULTS.strip_prefix("skills/orchestration/references/").unwrap()))
    );
    Ok(())
}

#[test]
fn distributed_delivery_contract_exposes_generic_primary_and_fallback() -> TestResult {
    let fixture = support::plugin_fixture_with_mutable_files(&[Path::new(POLICY)])?;
    let policy: Value = serde_json::from_str(&std::fs::read_to_string(fixture.root().join(POLICY))?)?;
    let parent_to_generic = policy
        .get("delivery")
        .and_then(Value::as_object)
        .and_then(|delivery| delivery.get("parent_to_generic"))
        .and_then(Value::as_object)
        .ok_or("parent-to-generic delivery contract")?;

    assert_eq!(
        parent_to_generic.get("primary"),
        Some(&json!({"model":"gpt-5.6-luna","thinking":"max"}))
    );
    assert_eq!(
        parent_to_generic.get("fallback"),
        Some(&json!({"model":"gpt-5.6-terra","thinking":"high"}))
    );
    Ok(())
}

#[test]
fn policy_rejects_unknown_fields_invalid_simple_evidence_and_unearned_promotion() -> TestResult {
    let fixture = support::plugin_fixture_with_mutable_files(&[Path::new(POLICY), Path::new(RESULTS)])?;
    let policy_path = fixture.root().join(POLICY);
    let mut policy: Value = serde_json::from_str(&std::fs::read_to_string(&policy_path)?)?;
    policy["unexpected"] = json!(true);
    std::fs::write(&policy_path, serde_json::to_vec(&policy)?)?;
    assert!(!check(fixture.root())?.status.success(), "unknown policy field passed");
    fixture.reset_file(Path::new(POLICY))?;
    let policy = std::fs::read_to_string(&policy_path)?;
    let duplicate = policy.replacen(
        "\"schema\": \"codexy.child-routing-policy.v1\"",
        "\"schema\": \"other\", \"schema\": \"codexy.child-routing-policy.v1\"",
        1,
    );
    std::fs::write(&policy_path, duplicate)?;
    assert!(!check(fixture.root())?.status.success(), "duplicate policy key passed");
    fixture.reset_file(Path::new(POLICY))?;

    let output = resolve(
        fixture.root(),
        json!({"schema":"codexy.child-routing-request.v1","classification":"simple","simple_predicates":{"fixed_scope":true,"deterministic_oracle":true,"low_risk_reversible":true,"no_unresolved_decision":false},"codex_thread_operation":"create_thread"}),
    )?;
    assert!(output.status.success(), "incomplete classification did not return a fail-closed route");
    assert_eq!(serde_json::from_slice::<Value>(&output.stdout)?, json!({"route":"root_or_named_specialist"}));
    let unknown = resolve(
        fixture.root(),
        json!({"schema":"codexy.child-routing-request.v1","classification":"general","named_specialist":"codexy-unknown","codex_thread_operation":"create_thread"}),
    )?;
    assert!(!unknown.status.success(), "unknown named specialist passed");

    let results_path = fixture.root().join(RESULTS);
    let mut results: Value = serde_json::from_str(&std::fs::read_to_string(&results_path)?)?;
    results["selected_effort"] = json!("max");
    std::fs::write(&results_path, serde_json::to_vec(&results)?)?;
    assert!(!check(fixture.root())?.status.success(), "null economics promoted Terra/max");
    Ok(())
}

#[test]
fn resolver_binds_generic_routes_to_codex_app_thread_operations() -> TestResult {
    let fixture = support::plugin_fixture_with_mutable_files(&[Path::new(POLICY), Path::new(RESULTS)])?;
    let predicates = json!({
        "fixed_scope": true,
        "deterministic_oracle": true,
        "low_risk_reversible": true,
        "no_unresolved_decision": true
    });
    assert_route(
        fixture.root(),
        json!({
            "schema":"codexy.child-routing-request.v1",
            "classification":"simple",
            "simple_predicates": predicates,
            "codex_thread_operation":"send_message_to_thread",
            "codex_thread_capabilities":{"models":[{"model":"gpt-5.6-terra","thinking":["high"]}]}
        }),
        json!({"route":"generic","codex_thread_operation":"send_message_to_thread","model":"gpt-5.6-terra","thinking":"high"}),
    )?;
    assert_route(
        fixture.root(),
        json!({
            "schema":"codexy.child-routing-request.v1",
            "classification":"simple",
            "simple_predicates": predicates,
            "codex_thread_operation":"create_thread",
            "codex_thread_capabilities":{"models":[
                {"model":"gpt-5.6-terra","thinking":["high"]},
                {"model":"gpt-5.6-luna","thinking":["max"]}
            ]}
        }),
        json!({"route":"generic","codex_thread_operation":"create_thread","model":"gpt-5.6-luna","thinking":"max"}),
    )?;
    assert_route(
        fixture.root(),
        json!({
            "schema":"codexy.child-routing-request.v1",
            "classification":"general",
            "codex_thread_operation":"create_thread",
            "codex_thread_capabilities":{"models":[]}
        }),
        json!({"route":"root_or_named_specialist"}),
    )?;
    let unknown_request_field = resolve(
        fixture.root(),
        json!({
            "schema":"codexy.child-routing-request.v1",
            "classification":"general",
            "codex_thread_operation":"create_thread",
            "unexpected":true
        }),
    )?;
    assert!(
        !unknown_request_field.status.success(),
        "generic routing accepted a request outside the closed Codex app thread contract"
    );
    let unsupported_operation = resolve(
        fixture.root(),
        json!({
            "schema":"codexy.child-routing-request.v1",
            "classification":"general",
            "codex_thread_operation":"unsupported_operation",
            "codex_thread_capabilities":{"models":[{"model":"gpt-5.6-terra","thinking":["high"]}]}
        }),
    )?;
    assert!(
        !unsupported_operation.status.success(),
        "generic routing accepted a non-Codex-thread operation"
    );
    let invalid_capabilities = resolve(
        fixture.root(),
        json!({
            "schema":"codexy.child-routing-request.v1",
            "classification":"general",
            "codex_thread_operation":"create_thread",
            "codex_thread_capabilities":{"models":[],"unexpected":true}
        }),
    )?;
    assert!(
        !invalid_capabilities.status.success(),
        "generic routing accepted malformed Codex app thread capabilities"
    );
    Ok(())
}

fn assert_route(root: &Path, request: Value, expected: Value) -> TestResult {
    let output = resolve(root, request)?;
    assert!(output.status.success(), "resolver failed: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(serde_json::from_slice::<Value>(&output.stdout)?, expected);
    Ok(())
}

fn resolve(root: &Path, request: Value) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let request_path = temp.path().join("request.json");
    std::fs::write(&request_path, serde_json::to_vec(&request)?)?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", root.to_str().ok_or("plugin root")?, "--resolve-child-routing", "--routing-request-file"])
        .arg(request_path)
        .output()?)
}

fn check(root: &Path) -> TestResult<std::process::Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", root.to_str().ok_or("plugin root")?, "--check"])
        .output()?)
}
