use serde_json::json;

use crate::support::TestResult;

use super::{fixtures, run_recovery, stderr};

#[test]
fn cli_recovery_projects_policy_and_source_reviewers() -> TestResult {
    let current = fixtures::recovery_snapshot();
    let capture = fixtures::read_thread_request();
    let (result, state) = run_recovery(&current, &capture)?;
    assert!(
        result.status.success(),
        "native history recovery failed: {}",
        stderr(&result)
    );
    let state = state.ok_or("successful recovery must write state")?;
    let control = &state["reviewControl"];
    assert_eq!(control["profile"], "strict");
    assert_eq!(control["reviewer"]["name"], "codexy-sentinel");
    assert_eq!(control["reviewer"]["model"], "gpt-6-astra");
    assert_eq!(control["full_review_count"], 1);
    assert_eq!(control["delta_review_count"], 1);
    assert_eq!(control["terminal_review_count"], 2);
    assert_eq!(
        control["terminal_review_history"][0]["source_reviewer"]["model"],
        "model.continuation"
    );
    assert_eq!(
        control["terminal_review_history"][1]["source_reviewer"]["model"],
        "model.initial"
    );
    assert_eq!(
        state["nativeHistoryRecovery"]["admission"]["temporal"],
        "proved_post_pr"
    );
    assert_eq!(
        state["nativeHistoryRecovery"]["admission"]["result"],
        "not_admitted"
    );
    assert_eq!(
        control["native_history_recovery"]["event_ids"],
        json!(["message-full", "message-delta"])
    );
    assert_eq!(
        state["nativeHistoryRecovery"]["source"]["owner"]["capture"],
        capture["owner"]["capture"]
    );
    assert_eq!(
        state["nativeHistoryRecovery"]["source"]["reviewer"]["capture"],
        capture["reviewer"]["capture"]
    );
    Ok(())
}
