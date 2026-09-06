use crate::support::TestResult;
use serde_json::json;

use super::{
    BASE_OID, HEAD_OID, NEXT_HEAD_OID, direct_state, run_build, snapshot,
};

#[test]
fn build_pr_state_preserves_history_across_graphql_and_connector_sources() -> TestResult {
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
    let current = direct_state::connector_pr_snapshot_without_derived(
        pr_number,
        issue_number,
        BASE_OID,
        NEXT_HEAD_OID,
        None,
    );
    let (result, state) = run_build(&current, &current_control, &previous)?;
    assert!(
        result.status.success(),
        "supported source transition must preserve valid history: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let state = state.expect("successful source transition must write PR state");
    assert_eq!(state["capture"]["method"], "connector");
    assert_eq!(state["reviewControl"]["terminal_review_count"], json!(2));
    assert_eq!(
        state["reviewControl"]["terminal_review_history"][0],
        previous_control["terminal_review_history"][0]
    );
    assert_eq!(
        state["reviewControl"]["terminal_review_history"][1],
        current_control["terminal_review_history"][1]
    );
    Ok(())
}
