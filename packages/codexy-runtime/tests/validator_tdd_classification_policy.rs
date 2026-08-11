use std::{path::Path, process::Command};

use serde_json::{Value, json};

use crate::support::{self, TestResult};

const POLICY: &str = "skills/orchestration/references/tdd-classification-policy.json";

#[test]
fn resolver_classifies_engineering_non_engineering_and_mixed_boundaries() -> TestResult {
    let fixture = support::plugin_fixture()?;
    for (boundaries, expected) in [
        (
            json!(["production_code"]),
            json!({"classification":"engineering","engineering_tdd_required":true,"tdd_boundaries":["production_code"],"proportional_proof_boundaries":[]}),
        ),
        (
            json!(["runtime_behavior", "validator", "hook", "cli", "workflow"]),
            json!({"classification":"engineering","engineering_tdd_required":true,"tdd_boundaries":["runtime_behavior","validator","hook","cli","workflow"],"proportional_proof_boundaries":[]}),
        ),
        (
            json!(["markdown_backed_parser"]),
            json!({"classification":"engineering","engineering_tdd_required":true,"tdd_boundaries":["markdown_backed_parser"],"proportional_proof_boundaries":[]}),
        ),
        (
            json!(["readme", "documentation", "instruction_only_skill"]),
            json!({"classification":"non_engineering","engineering_tdd_required":false,"tdd_boundaries":[],"proportional_proof_boundaries":["readme","documentation","instruction_only_skill"]}),
        ),
        (
            json!(["declarative_metadata", "diagram", "roadmap_or_release_prose"]),
            json!({"classification":"non_engineering","engineering_tdd_required":false,"tdd_boundaries":[],"proportional_proof_boundaries":["declarative_metadata","diagram","roadmap_or_release_prose"]}),
        ),
        (
            json!(["validator", "documentation"]),
            json!({"classification":"mixed","engineering_tdd_required":true,"tdd_boundaries":["validator"],"proportional_proof_boundaries":["documentation"]}),
        ),
    ] {
        assert_resolution(fixture.root(), boundaries, expected)?;
    }
    Ok(())
}

#[test]
fn resolver_rejects_unknown_or_incomplete_machine_owned_requests() -> TestResult {
    let fixture = support::plugin_fixture()?;
    for request in [
        json!({"schema":"codexy.tdd-classification-request.v1","boundaries":[]}),
        json!({"schema":"codexy.tdd-classification-request.v1","boundaries":["markdown"]}),
        json!({"schema":"codexy.tdd-classification-request.v1","boundaries":["documentation","markdown"]}),
        json!({"schema":"codexy.tdd-classification-request.v1","boundaries":["documentation","documentation"]}),
        json!({"schema":"codexy.tdd-classification-request.v1","boundaries":["documentation"],"unexpected":true}),
        json!({"schema":"other","boundaries":["documentation"]}),
    ] {
        let output = resolve(fixture.root(), request)?;
        assert!(!output.status.success(), "invalid classification request passed");
    }
    Ok(())
}

#[test]
fn check_rejects_a_malformed_or_unknown_policy_boundary() -> TestResult {
    let fixture = support::plugin_fixture()?;
    let path = fixture.root().join(POLICY);
    let mut policy = json!({
        "schema": "codexy.tdd-classification-policy.v1",
        "engineering_boundaries": ["production_code"],
        "non_engineering_boundaries": ["documentation"]
    });
    std::fs::write(&path, serde_json::to_vec(&policy)?)?;
    assert!(
        !check(fixture.root())?.status.success(),
        "incomplete policy passed"
    );
    policy["engineering_boundaries"] = json!(["documentation"]);
    std::fs::write(&path, serde_json::to_vec(&policy)?)?;
    assert!(!check(fixture.root())?.status.success(), "invalid policy passed");
    policy = json!({
        "schema": "codexy.tdd-classification-policy.v1",
        "engineering_boundaries": ["production_code"],
        "non_engineering_boundaries": ["documentation"],
        "unexpected": true
    });
    std::fs::write(&path, serde_json::to_vec(&policy)?)?;
    assert!(
        !check(fixture.root())?.status.success(),
        "unknown policy field passed"
    );
    Ok(())
}

fn assert_resolution(root: &Path, boundaries: Value, expected: Value) -> TestResult {
    let output = resolve(
        root,
        json!({"schema":"codexy.tdd-classification-request.v1","boundaries":boundaries}),
    )?;
    assert!(
        output.status.success(),
        "resolver failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(serde_json::from_slice::<Value>(&output.stdout)?, expected);
    Ok(())
}

fn resolve(root: &Path, request: Value) -> TestResult<std::process::Output> {
    let temporary = tempfile::tempdir()?;
    let request_path = temporary.path().join("request.json");
    std::fs::write(&request_path, serde_json::to_vec(&request)?)?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            root.to_str().ok_or("plugin root")?,
            "--resolve-tdd-classification",
            "--tdd-classification-request-file",
        ])
        .arg(request_path)
        .output()?)
}

fn check(root: &Path) -> TestResult<std::process::Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", root.to_str().ok_or("plugin root")?, "--check"])
        .output()?)
}
