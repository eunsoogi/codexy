use std::{path::Path, process::Command};

use serde_json::{Value, json};

use crate::support::TestResult;

pub(super) const BEHAVIOR: &[&str] = &["requirement_linked_behavioral_verification"];

pub(super) fn cli_root() -> TestResult<tempfile::TempDir> {
    Ok(tempfile::tempdir()?)
}

fn resolve(request: Value) -> TestResult<Value> {
    resolve_text(&serde_json::to_string(&request)?)
}

fn resolve_text(request: &str) -> TestResult<Value> {
    // The production resolver currently does not read plugin_root, so the
    // repository's actual path is sufficient without copying a fixture.
    let plugin_root = codexy_runtime::paths::plugin_root();
    Ok(codexy_runtime::validation::resolve_tdd_classification(
        &plugin_root,
        request,
    )?)
}

fn resolve_cli(root: &Path, request: Value) -> TestResult<std::process::Output> {
    resolve_cli_text(root, &serde_json::to_string(&request)?)
}

fn resolve_cli_text(root: &Path, request: &str) -> TestResult<std::process::Output> {
    let temporary = tempfile::tempdir()?;
    let request_path = temporary.path().join("request.json");
    std::fs::write(&request_path, request.as_bytes())?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            root.to_str().ok_or("plugin root")?,
            "--resolve-tdd-classification",
            "--tdd-classification-request-file",
        ])
        .arg(request_path)
        .output()?;
    Ok(output)
}

pub(super) fn assert_v1(boundaries: Value, expected: Value) -> TestResult {
    assert_eq!(
        resolve(
            json!({"schema":"codexy.tdd-classification-request.v1","boundaries":boundaries}),
        )?,
        expected
    );
    Ok(())
}

pub(super) fn assert_v1_cli(
    root: &Path,
    boundaries: Value,
    expected: Value,
) -> TestResult {
    let output = resolve_cli(
        root,
        json!({"schema":"codexy.tdd-classification-request.v1","boundaries":boundaries}),
    )?;
    assert_output(&output, expected, "v1 CLI resolver failed")
}

pub(super) fn assert_v2(request: Value, expected: Value) -> TestResult {
    assert_eq!(resolve(request)?, expected);
    Ok(())
}

pub(super) fn assert_v2_cli(
    root: &Path,
    request: Value,
    expected: Value,
) -> TestResult {
    let output = resolve_cli(root, request)?;
    assert_output(&output, expected, "v2 CLI resolver failed")
}

pub(super) fn assert_rejected(request: Value, message: &str) -> TestResult {
    let result = resolve(request);
    assert!(result.is_err(), "{message}: {result:?}");
    Ok(())
}

pub(super) fn assert_text_rejected(request: &str, message: &str) -> TestResult {
    let result = resolve_text(request);
    assert!(result.is_err(), "{message}: {result:?}");
    Ok(())
}

pub(super) fn assert_cli_rejected(
    root: &Path,
    request: Value,
    expected_error: &str,
    message: &str,
) -> TestResult {
    let output = resolve_cli(root, request)?;
    assert!(!output.status.success(), "{message}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected_error),
        "{message}: expected stderr to contain {expected_error:?}, got {stderr}"
    );
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

fn assert_output(
    output: &std::process::Output,
    expected: Value,
    message: &str,
) -> TestResult {
    assert_success(output, message);
    assert_eq!(serde_json::from_slice::<Value>(&output.stdout)?, expected);
    Ok(())
}
