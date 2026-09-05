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
