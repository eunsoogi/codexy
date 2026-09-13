use crate::support::TestResult;

use super::{
    BASE_OID, HEAD_OID, NEXT_HEAD_OID, direct_state, run_build, snapshot,
};

#[test]
fn build_pr_state_accepts_a_connector_snapshot_without_history() -> TestResult {
    let issue_number = 11;
    let pr_number = 17;
    let current_control = direct_state::strict_control(issue_number, NEXT_HEAD_OID);
    let previous = snapshot(
        pr_number,
        issue_number,
        BASE_OID,
        HEAD_OID,
        Some(direct_state::strict_control(issue_number, HEAD_OID)),
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
        "supported connector snapshot must accept current-head review: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let state = state.expect("successful source transition must write PR state");
    assert_eq!(state["capture"]["method"], "connector");
    assert_eq!(state["reviewControl"], current_control);
    Ok(())
}
