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
        json!({"schema":"codexy.child-routing-request.v1","classification":"general","recipient_capabilities":{"models":[{"model":"gpt-5.6-terra","thinking":["high"]}]}}),
        json!({"route":"generic","model":"gpt-5.6-terra","thinking":"high"}),
    )?;
    assert_route(
        fixture.root(),
        json!({"schema":"codexy.child-routing-request.v1","classification":"simple","simple_predicates":{"fixed_scope":true,"deterministic_oracle":true,"low_risk_reversible":true,"no_unresolved_decision":true},"recipient_capabilities":{"models":[{"model":"gpt-5.6-luna","thinking":["max"]}]}}),
        json!({"route":"generic","model":"gpt-5.6-luna","thinking":"max"}),
    )?;
    assert_route(
        fixture.root(),
        json!({"schema":"codexy.child-routing-request.v1","classification":"general","named_specialist":"codexy-architect"}),
        json!({"route":"named_specialist","agent_type":"codexy-architect"}),
    )?;
    assert_route(
        fixture.root(),
        json!({"schema":"codexy.child-routing-request.v1","classification":"ambiguous"}),
        json!({"route":"root_or_named_specialist"}),
    )?;
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
        json!({"schema":"codexy.child-routing-request.v1","classification":"simple","simple_predicates":{"fixed_scope":true,"deterministic_oracle":true,"low_risk_reversible":true,"no_unresolved_decision":false}}),
    )?;
    assert!(output.status.success(), "incomplete classification did not return a fail-closed route");
    assert_eq!(serde_json::from_slice::<Value>(&output.stdout)?, json!({"route":"root_or_named_specialist"}));
    let unknown = resolve(
        fixture.root(),
        json!({"schema":"codexy.child-routing-request.v1","classification":"general","named_specialist":"codexy-unknown"}),
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
fn resolver_requires_structured_recipient_capabilities_for_generic_routes() -> TestResult {
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
            "recipient_capabilities":{"models":[{"model":"gpt-5.6-terra","thinking":["high"]}]}
        }),
        json!({"route":"generic","model":"gpt-5.6-terra","thinking":"high"}),
    )?;
    assert_route(
        fixture.root(),
        json!({
            "schema":"codexy.child-routing-request.v1",
            "classification":"simple",
            "simple_predicates": predicates,
            "recipient_capabilities":{"models":[
                {"model":"gpt-5.6-terra","thinking":["high"]},
                {"model":"gpt-5.6-luna","thinking":["max"]}
            ]}
        }),
        json!({"route":"generic","model":"gpt-5.6-luna","thinking":"max"}),
    )?;
    assert_route(
        fixture.root(),
        json!({
            "schema":"codexy.child-routing-request.v1",
            "classification":"general",
            "recipient_capabilities":{"models":[]}
        }),
        json!({"route":"root_or_named_specialist"}),
    )?;
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
