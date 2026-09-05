use std::{fs, path::Path, process::Command};

use crate::support::TestResult;
use serde_json::{Value, json};

#[path = "support/review_control_direct_state.rs"]
mod direct_state;

const BASE_OID: &str = "0000000000000000000000000000000000000001";
const HEAD_OID: &str = "0000000000000000000000000000000000000002";
const PR_NUMBER: u64 = 17;
const ISSUE_NUMBER: u64 = 11;

#[test]
fn completion_handoff_accepts_authenticated_unequal_pr_and_issue() -> TestResult {
    let output = validate(pr_state())?;
    assert!(
        output.status.success(),
        "valid unequal PR/issue state must pass: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn completion_handoff_rejects_missing_capture() -> TestResult {
    let mut state = pr_state();
    state
        .as_object_mut()
        .expect("PR state object")
        .remove("capture");
    let output = validate(state)?;
    assert!(!output.status.success(), "missing PR capture must block");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("capture provenance"),
        "missing capture diagnostic must name the missing provenance: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn completion_handoff_rejects_owning_issue_substitution() -> TestResult {
    let mut state = pr_state();
    state["capture"]["owningIssue"]["number"] = json!(12);
    state["capture"]["owningIssue"]["url"] = json!(
        "https://github.com/eunsoogi/codexy/issues/12"
    );
    let output = validate(state)?;
    assert!(!output.status.success(), "issue substitution must block");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("owning issue"),
        "issue substitution diagnostic must name the owning issue: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let mut state = pr_state();
    state["reviewControl"]["issue_number"] = json!(12);
    let output = validate(state)?;
    assert!(!output.status.success(), "control issue substitution must block");
    Ok(())
}

#[test]
fn completion_handoff_accepts_authenticated_light_pr_snapshot() -> TestResult {
    let output = validate(light_pr_state())?;
    assert!(
        output.status.success(),
        "valid light PR snapshot must pass: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn completion_handoff_rejects_light_snapshot_identity_bypass() -> TestResult {
    let mut missing_capture = light_pr_state();
    missing_capture
        .as_object_mut()
        .expect("light PR state object")
        .remove("capture");
    let output = validate(missing_capture)?;
    assert!(!output.status.success(), "light capture omission must block");
    assert!(String::from_utf8_lossy(&output.stderr).contains("capture provenance"));

    let mut invalid_head = light_pr_state();
    invalid_head["headRefOid"] = json!("not-a-sha");
    let output = validate(invalid_head)?;
    assert!(!output.status.success(), "light invalid head must block");
    assert!(String::from_utf8_lossy(&output.stderr).contains("headRefOid"));

    let mut missing_repository = light_pr_state();
    missing_repository
        .as_object_mut()
        .expect("light PR state object")
        .remove("repository");
    let output = validate(missing_repository)?;
    assert!(!output.status.success(), "light missing repository must block");
    assert!(String::from_utf8_lossy(&output.stderr).contains("repository"));

    let mut missing_number = light_pr_state();
    missing_number
        .as_object_mut()
        .expect("light PR state object")
        .remove("number");
    let output = validate(missing_number)?;
    assert!(!output.status.success(), "light missing PR number must block");
    assert!(String::from_utf8_lossy(&output.stderr).contains("number"));

    let mut invalid_base = light_pr_state();
    invalid_base["baseRefOid"] = json!("not-a-sha");
    let output = validate(invalid_base)?;
    assert!(!output.status.success(), "light invalid base must block");
    assert!(String::from_utf8_lossy(&output.stderr).contains("baseRefOid"));
    Ok(())
}

#[test]
fn build_pr_state_requires_authenticated_light_pr_snapshot() -> TestResult {
    let valid = build(light_pr_state())?;
    assert!(
        valid.status.success(),
        "valid light PR snapshot must build: {}",
        String::from_utf8_lossy(&valid.stderr)
    );

    let mut missing_capture = light_pr_state();
    missing_capture
        .as_object_mut()
        .expect("light PR state object")
        .remove("capture");
    let output = build(missing_capture)?;
    assert!(!output.status.success(), "light build without capture must block");
    assert!(String::from_utf8_lossy(&output.stderr).contains("capture provenance"));

    let mut invalid_head = light_pr_state();
    invalid_head["headRefOid"] = json!("not-a-sha");
    let output = build(invalid_head)?;
    assert!(!output.status.success(), "light build with invalid head must block");
    assert!(String::from_utf8_lossy(&output.stderr).contains("headRefOid"));
    Ok(())
}

fn pr_state() -> Value {
    let mut state = direct_state::pr_snapshot(
        PR_NUMBER,
        BASE_OID,
        HEAD_OID,
        Some(direct_state::strict_control(ISSUE_NUMBER, HEAD_OID)),
    );
    state["capture"]["owningIssue"]["number"] = json!(ISSUE_NUMBER);
    state["capture"]["owningIssue"]["url"] = json!(format!(
        "https://github.com/eunsoogi/codexy/issues/{ISSUE_NUMBER}"
    ));
    state["state"] = json!("OPEN");
    state["isDraft"] = json!(true);
    state["mergeStateStatus"] = json!("CLEAN");
    state["reviewProfile"] = json!("strict");
    state
}

fn light_pr_state() -> Value {
    let mut state = direct_state::pr_snapshot(PR_NUMBER, BASE_OID, HEAD_OID, None);
    state["capture"]["owningIssue"]["number"] = json!(ISSUE_NUMBER);
    state["capture"]["owningIssue"]["url"] = json!(format!(
        "https://github.com/eunsoogi/codexy/issues/{ISSUE_NUMBER}"
    ));
    state["state"] = json!("OPEN");
    state["isDraft"] = json!(true);
    state["mergeStateStatus"] = json!("CLEAN");
    state["reviewProfile"] = json!("light");
    state["reviewControl"] = json!({
        "schema": "codexy.review-control-state.v1",
        "profile": "light"
    });
    state
}

fn validate(state: Value) -> TestResult<std::process::Output> {
    let temporary = tempfile::tempdir()?;
    let handoff = temporary.path().join("handoff.md");
    let pr_state = temporary.path().join("pr-state.json");
    fs::write(&handoff, "임의의 prose와 순서입니다.\n")?;
    fs::write(&pr_state, serde_json::to_vec(&state)?)?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-completion-handoff", "--handoff-file"])
        .arg(&handoff)
        .args(["--pr-state-file"])
        .arg(&pr_state)
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")))
        .output()?)
}

fn build(state: Value) -> TestResult<std::process::Output> {
    let temporary = tempfile::tempdir()?;
    let current = temporary.path().join("current-pr-state.json");
    let control = temporary.path().join("review-control.json");
    let previous = temporary.path().join("previous-pr-state.json");
    let output = temporary.path().join("pr-state.json");
    let mut current_state = state;
    let control_state = current_state["reviewControl"].clone();
    current_state
        .as_object_mut()
        .expect("light PR state object")
        .remove("reviewControl");
    fs::write(&current, serde_json::to_vec(&current_state)?)?;
    fs::write(&control, serde_json::to_vec(&control_state)?)?;
    fs::write(&previous, serde_json::to_vec(&light_pr_state())?)?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--build-pr-state", "--base-pr-state-file"])
        .arg(&current)
        .args(["--review-control-state-file"])
        .arg(&control)
        .args(["--previous-pr-state-file"])
        .arg(&previous)
        .args(["--output"])
        .arg(&output)
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")))
        .output()?)
}
