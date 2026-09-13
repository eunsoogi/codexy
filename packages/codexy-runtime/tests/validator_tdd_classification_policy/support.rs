use std::{path::Path, process::Command};

use serde_json::{Value, json};

use crate::support::{self, TestResult};

pub(super) const BEHAVIOR: &[&str] = &["requirement_linked_behavioral_verification"];

pub(super) fn fixture() -> TestResult<support::PluginFixture> {
    Ok(support::plugin_fixture()?)
}

pub(super) fn resolve(root: &Path, request: Value) -> TestResult<std::process::Output> {
    resolve_text(root, &serde_json::to_string(&request)?)
}

pub(super) fn resolve_text(root: &Path, request: &str) -> TestResult<std::process::Output> {
    let temporary = tempfile::tempdir()?;
    let request_path = temporary.path().join("request.json");
    std::fs::write(&request_path, request.as_bytes())?;
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

pub(super) fn assert_v1(root: &Path, boundaries: Value, expected: Value) -> TestResult {
    let output = resolve(
        root,
        json!({"schema":"codexy.tdd-classification-request.v1","boundaries":boundaries}),
    )?;
    assert_success(&output, "v1 resolver failed");
    assert_eq!(serde_json::from_slice::<Value>(&output.stdout)?, expected);
    Ok(())
}

pub(super) fn assert_v2(root: &Path, request: Value, expected: Value) -> TestResult {
    let output = resolve(root, request)?;
    assert_success(&output, "v2 resolver failed");
    assert_eq!(serde_json::from_slice::<Value>(&output.stdout)?, expected);
    Ok(())
}

pub(super) fn assert_rejected(root: &Path, request: Value, message: &str) -> TestResult {
    let output = resolve(root, request)?;
    assert!(!output.status.success(), "{message}");
    Ok(())
}

pub(super) fn assert_text_rejected(root: &Path, request: &str, message: &str) -> TestResult {
    let output = resolve_text(root, request)?;
    assert!(!output.status.success(), "{message}");
    Ok(())
}

pub(super) fn request(boundaries: Vec<Value>) -> Value {
    json!({"schema":"codexy.tdd-classification-request.v2","boundaries":boundaries})
}

pub(super) fn boundary(
    id: &str,
    kind: &str,
    purpose: &str,
    risks: &[&str],
    test_first_required: bool,
    reproduction: Option<Value>,
) -> Value {
    let mut change = json!({"purpose":purpose});
    if let Some(reproduction) = reproduction {
        change["reproduction"] = reproduction;
    }
    json!({"id":id,"kind":kind,"change":change,"risks":risks,"test_first_required":test_first_required})
}

pub(super) fn expected(
    classification: &str,
    tests_required: bool,
    mode: &str,
    tdd_boundaries: &[&str],
    proof_boundaries: &[&str],
    obligations: Vec<Value>,
) -> Value {
    json!({"schema":"codexy.tdd-classification-result.v2","classification":classification,"engineering_tests_required":tests_required,"tdd_mode":mode,"tdd_boundaries":tdd_boundaries,"proportional_proof_boundaries":proof_boundaries,"boundary_obligations":obligations})
}

pub(super) fn obligation(
    id: &str,
    kind: &str,
    tests_required: bool,
    mode: &str,
    pre_change: &[&str],
    proof: &[&str],
) -> Value {
    json!({"id":id,"kind":kind,"engineering_tests_required":tests_required,"tdd_mode":mode,"pre_change_obligations":pre_change,"proof_obligations":proof})
}

fn assert_success(output: &std::process::Output, message: &str) {
    assert!(
        output.status.success(),
        "{message}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
