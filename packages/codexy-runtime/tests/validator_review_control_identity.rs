use std::fs;

use crate::support::{FixtureCommand, TestResult};
use serde_json::{Value, json};

#[path = "support/review_control_direct_state.rs"]
mod direct_state;

const BASE_OID: &str = "0000000000000000000000000000000000000001";
const HEAD_OID: &str = "0000000000000000000000000000000000000002";
const NEXT_HEAD_OID: &str = "0000000000000000000000000000000000000003";

#[test]
fn build_pr_state_accepts_unequal_authenticated_issue_and_pr_numbers() -> TestResult {
    let issue_number = 11;
    let pr_number = 17;
    let current = snapshot(pr_number, issue_number, BASE_OID, HEAD_OID, None);
    let previous = snapshot(
        pr_number,
        issue_number,
        BASE_OID,
        BASE_OID,
        Some(direct_state::strict_genesis(issue_number)),
    );
    let (result, state) = run_build(
        &current,
        &direct_state::strict_control(issue_number, HEAD_OID),
        &previous,
    )?;
    assert!(
        result.status.success(),
        "unequal issue/PR identities must be accepted: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let state = state.expect("successful build must write PR state");
    assert_eq!(state["number"], json!(pr_number));
    assert_eq!(state["url"], format!("https://github.com/eunsoogi/codexy/pull/{pr_number}"));
    assert_eq!(state["capture"]["owningIssue"]["number"], json!(issue_number));
    assert_eq!(state["reviewControl"]["issue_number"], json!(issue_number));
    Ok(())
}

#[test]
fn build_pr_state_preserves_history_for_unequal_issue_and_pr_numbers() -> TestResult {
    let issue_number = 11;
    let pr_number = 17;
    let previous_control = direct_state::strict_control(issue_number, HEAD_OID);
    let mut current_control = direct_state::strict_control(issue_number, NEXT_HEAD_OID);
    current_control["delta_review_count"] = json!(1);
    current_control["terminal_review_count"] = json!(2);
    current_control["terminal_review_history"] = json!([
        direct_state::review_event("strict-full-1", "full", HEAD_OID, "PASS"),
        direct_state::review_event("strict-delta-1", "delta", NEXT_HEAD_OID, "PASS")
    ]);
    let previous = snapshot(
        pr_number,
        issue_number,
        BASE_OID,
        HEAD_OID,
        Some(previous_control.clone()),
    );
    let current = snapshot(pr_number, issue_number, BASE_OID, NEXT_HEAD_OID, None);
    let (result, state) = run_build(&current, &current_control, &previous)?;
    assert!(
        result.status.success(),
        "unequal identities must preserve a valid history transition: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let state = state.expect("successful transition must write PR state");
    assert_eq!(state["number"], json!(pr_number));
    assert_eq!(state["capture"]["owningIssue"]["number"], json!(issue_number));
    assert_eq!(state["reviewControl"]["terminal_review_count"], json!(2));
    assert_eq!(
        state["reviewControl"]["terminal_review_history"][0],
        previous_control["terminal_review_history"][0]
    );
    Ok(())
}

#[test]
fn build_pr_state_rejects_missing_or_tampered_issue_capture() -> TestResult {
    let issue_number = 11;
    let pr_number = 17;
    let current = snapshot(pr_number, issue_number, BASE_OID, HEAD_OID, None);
    let previous = snapshot(
        pr_number,
        issue_number,
        BASE_OID,
        BASE_OID,
        Some(direct_state::strict_genesis(issue_number)),
    );
    let control = direct_state::strict_control(issue_number, HEAD_OID);
    let mut cases = Vec::new();

    let mut missing = current.clone();
    missing["capture"]
        .as_object_mut()
        .expect("capture object")
        .remove("owningIssue");
    cases.push(("missing issue evidence", missing, previous.clone(), control.clone()));

    let mut unauthenticated = current.clone();
    unauthenticated["capture"]["authenticated"] = json!(false);
    cases.push(("unauthenticated capture", unauthenticated, previous.clone(), control.clone()));

    let mut wrong_url = current.clone();
    wrong_url["capture"]["owningIssue"]["url"] = json!(
        "https://github.com/eunsoogi/codexy/pull/11"
    );
    cases.push(("wrong issue URL", wrong_url, previous.clone(), control.clone()));

    let mut wrong_repository = current.clone();
    wrong_repository["capture"]["owningIssue"]["repository"] = json!("other/codexy");
    cases.push(("wrong issue repository", wrong_repository, previous.clone(), control.clone()));

    let mut arbitrary_association = current.clone();
    arbitrary_association["capture"]["owningIssue"]["association"] = json!("body-parse");
    cases.push((
        "untrusted association",
        arbitrary_association,
        previous.clone(),
        control.clone(),
    ));

    let mismatched_previous = snapshot(
        pr_number,
        issue_number + 1,
        BASE_OID,
        BASE_OID,
        Some(direct_state::strict_genesis(issue_number)),
    );
    cases.push((
        "changed owning issue",
        current.clone(),
        mismatched_previous,
        control.clone(),
    ));

    cases.push((
        "control issue mismatch",
        current,
        previous,
        direct_state::strict_control(issue_number + 1, HEAD_OID),
    ));

    for (label, current, previous, control) in cases {
        let (result, _) = run_build(&current, &control, &previous)?;
        assert!(
            !result.status.success(),
            "{label} must be rejected; stderr: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    Ok(())
}

fn snapshot(
    pr_number: u64,
    issue_number: u64,
    base: &str,
    head: &str,
    control: Option<Value>,
) -> Value {
    let mut snapshot = direct_state::pr_snapshot(pr_number, base, head, control);
    snapshot["capture"]["owningIssue"]["number"] = json!(issue_number);
    snapshot["capture"]["owningIssue"]["url"] = json!(format!(
        "https://github.com/eunsoogi/codexy/issues/{issue_number}"
    ));
    snapshot
}

fn run_build(
    current: &Value,
    control: &Value,
    previous: &Value,
) -> TestResult<(std::process::Output, Option<Value>)> {
    let temporary = tempfile::tempdir()?;
    let current_path = temporary.path().join("current-pr-state.json");
    let control_path = temporary.path().join("review-control.json");
    let previous_path = temporary.path().join("previous-pr-state.json");
    let output_path = temporary.path().join("pr-state.json");
    fs::write(&current_path, serde_json::to_vec(current)?)?;
    fs::write(&control_path, serde_json::to_vec(control)?)?;
    fs::write(&previous_path, serde_json::to_vec(previous)?)?;
    let mut command = FixtureCommand::new(
        codexy_runtime::paths::repository_root().join("scripts/build-pr-state"),
    );
    command
        .arg("--base-pr-state-file")
        .arg_path(&current_path)
        .arg("--review-control-state-file")
        .arg_path(&control_path)
        .arg("--previous-pr-state-file")
        .arg_path(&previous_path)
        .arg("--output")
        .arg_path(&output_path)
        .env_path(
            "CODEXY_REVIEW_CONTROL_BIN",
            env!("CARGO_BIN_EXE_codexy-review-control"),
        );
    let result = command.output()?;
    let state = result
        .status
        .success()
        .then(|| fs::read(&output_path))
        .transpose()?
        .map(|bytes| serde_json::from_slice(&bytes))
        .transpose()?;
    Ok((result, state))
}
