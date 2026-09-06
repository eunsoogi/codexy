use std::fs;

use crate::support::TestResult;
use serde_json::json;

use super::{BASE_OID, HEAD_OID, direct_state, run_capture};

#[test]
fn build_pr_state_accepts_supported_connector_snapshot_with_provenance() -> TestResult {
    let issue_number = 11;
    let pr_number = 17;
    let current = direct_state::connector_pr_snapshot_without_derived(
        pr_number,
        issue_number,
        BASE_OID,
        HEAD_OID,
        None,
    );
    let previous = direct_state::connector_pr_snapshot_without_derived(
        pr_number,
        issue_number,
        BASE_OID,
        BASE_OID,
        Some(direct_state::strict_genesis(issue_number)),
    );
    let (result, state) = run_snapshot_build(
        &current,
        &direct_state::strict_control(issue_number, HEAD_OID),
        &previous,
    )?;
    assert!(
        result.status.success(),
        "supported connector snapshot must be accepted: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let state = state.expect("successful connector build must write PR state");
    assert_eq!(state["capture"]["method"], "connector");
    assert_eq!(
        state["capture"]["source"]["tool"],
        "mcp__codex_apps__github_get_pr_info"
    );
    assert_eq!(
        state["capture"]["source"]["arguments"]["repository_full_name"],
        "eunsoogi/codexy"
    );
    assert_eq!(
        state["capture"]["source"]["result"]["title"],
        "connector fixture"
    );
    assert_eq!(state["number"], json!(pr_number));
    assert_eq!(state["capture"]["owningIssue"]["number"], json!(issue_number));
    Ok(())
}

#[test]
fn build_pr_state_rejects_connector_provenance_gaps_and_identity_changes() -> TestResult {
    let issue_number = 11;
    let pr_number = 17;
    let current = direct_state::connector_pr_snapshot(
        pr_number,
        issue_number,
        BASE_OID,
        HEAD_OID,
        None,
    );
    let previous = direct_state::pr_snapshot(
        pr_number,
        BASE_OID,
        BASE_OID,
        Some(direct_state::strict_genesis(issue_number)),
    );
    let control = direct_state::strict_control(issue_number, HEAD_OID);
    let mut cases = Vec::new();

    let mut unsupported = current.clone();
    unsupported["capture"]["method"] = json!("rest");
    cases.push(("unsupported connector method", unsupported));

    let mut missing_source = current.clone();
    missing_source["capture"]
        .as_object_mut()
        .expect("connector capture")
        .remove("source");
    cases.push(("missing connector source", missing_source));

    let mut missing_issue = current.clone();
    missing_issue["capture"]
        .as_object_mut()
        .expect("connector capture")
        .remove("owningIssue");
    cases.push(("missing owning issue", missing_issue));

    let mut changed_result = current.clone();
    changed_result["capture"]["source"]["result"]["head_sha"] = json!(BASE_OID);
    cases.push(("contradictory connector result", changed_result));

    let mut changed_arguments = current.clone();
    changed_arguments["capture"]["source"]["arguments"]["pr_number"] = json!(18);
    cases.push(("contradictory connector arguments", changed_arguments));

    for (label, current) in cases {
        let (result, _) = run_snapshot_build(&current, &control, &previous)?;
        assert!(
            !result.status.success(),
            "{label} must be rejected; stderr: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    Ok(())
}

fn run_snapshot_build(
    current: &serde_json::Value,
    control: &serde_json::Value,
    previous: &serde_json::Value,
) -> TestResult<(std::process::Output, Option<serde_json::Value>)> {
    let temporary = tempfile::tempdir()?;
    let current_path = temporary.path().join("current-pr-state.json");
    let control_path = temporary.path().join("review-control.json");
    let previous_path = temporary.path().join("previous-pr-state.json");
    let output_path = temporary.path().join("pr-state.json");
    fs::write(&current_path, serde_json::to_vec(current)?)?;
    fs::write(&control_path, serde_json::to_vec(control)?)?;
    fs::write(&previous_path, serde_json::to_vec(previous)?)?;
    let result = run_capture(&current_path, &control_path, &previous_path, &output_path)?;
    let state = result
        .status
        .success()
        .then(|| fs::read(&output_path))
        .transpose()?
        .map(|bytes| serde_json::from_slice(&bytes))
        .transpose()?;
    Ok((result, state))
}
