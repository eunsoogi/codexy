use crate::support::TestResult;
#[path = "support/review_control_direct_state.rs"]
mod direct_state;
#[path = "support/final_disposition.rs"]
mod final_support;

#[test]
fn current_919_witness_consumes_sanitized_same_head_disposition() -> TestResult {
    let (recovered, current) = final_support::recover_919()?;
    let control = final_support::final_control_919(&recovered)?;
    let baseline = final_support::produce_with_recovered_predecessor(
        &control,
        &current,
        &recovered,
        final_support::native_919_pull_request,
    )?;
    assert!(!baseline.status.success());
    let baseline_diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&baseline.stdout),
        String::from_utf8_lossy(&baseline.stderr)
    );
    assert!(baseline_diagnostic.contains("only valid after the third terminal verdict"));
    let produced = final_support::produce_with_states(
        &control,
        &current,
        &recovered,
        final_support::native_919_pull_request,
    )?;
    let state = final_support::build_pr_state_with_states(
        &control,
        &current,
        &recovered,
        final_support::native_919_pull_request,
    )?;

    assert_eq!(produced["issue_number"], final_support::native_919_issue);
    assert_eq!(produced["terminal_review_count"], 3);
    assert_eq!(produced["terminal_review_history"].as_array().map(Vec::len), Some(3));
    assert_eq!(produced["native_history_provenance"]["event_ids"],
        serde_json::json!([final_support::native_919_full_event, final_support::native_919_delta_event]));
    assert_eq!(produced["terminal_review_history"][0]["id"], final_support::native_919_full_event);
    assert_eq!(produced["terminal_review_history"][1]["id"], final_support::native_919_delta_event);
    assert_eq!(produced["terminal_review_history"][2]["id"], final_support::native_919_required_event);
    assert_eq!(produced["terminal_review_history"][0]["reviewer"]["model"], "gpt-5.6-sol");
    assert_eq!(produced["terminal_review_history"][0]["policy_reviewer"]["model"], "gpt-6-astra");
    assert!(!produced["terminal_review_history"][0]["unresolved_findings"].as_array().unwrap().is_empty());
    assert!(!produced["terminal_review_history"][1]["unresolved_findings"].as_array().unwrap().is_empty());
    assert_eq!(produced["terminal_review_history"][2]["unresolved_findings"][0]["id"], final_support::native_919_remaining_finding);
    assert_eq!(state["nativeHistoryRecovery"], recovered["nativeHistoryRecovery"]);
    assert_eq!(state["reviewControl"]["terminal_review_history"], produced["terminal_review_history"]);
    assert!(state["reviewControl"].get("native_history_recovery").is_none());
    let handoff = final_support::validate_handoff_state(
        &state,
        final_support::native_919_pull_request,
    )?;
    assert!(handoff.status.success(), "#919 native-history handoff failed: {}", String::from_utf8_lossy(&handoff.stderr));
    Ok(())
}

#[test]
fn direct_handoff_rejects_unobservable_final_disposition() -> TestResult {
    let mut control = final_support::same_head_control(993);
    control["terminal_result"] = serde_json::json!("UNOBSERVABLE");
    control["terminal_review_history"][2]["terminal_result"] = serde_json::json!("UNOBSERVABLE");
    let handoff = final_support::validate_handoff_with_state(
        &control,
        993,
        direct_state::SYNTHETIC_CURRENT_HEAD,
        |state| {
            state["reviewControl"]
                .as_object_mut()
                .ok_or("direct handoff review control")?
                .remove("final_disposition");
            Ok(())
        },
    )?;
    assert!(!handoff.status.success());
    let diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&handoff.stdout),
        String::from_utf8_lossy(&handoff.stderr)
    );
    assert!(
        diagnostic.contains("terminal_result is not PASS"),
        "UNOBSERVABLE handoff was rejected for an unexpected reason: {diagnostic}"
    );
    Ok(())
}
