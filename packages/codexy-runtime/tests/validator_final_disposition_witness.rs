use serde_json::Value;

use crate::support::TestResult;
#[path = "support/review_control_direct_state.rs"]
mod direct_state;
#[path = "support/final_disposition.rs"]
mod final_support;

#[test]
fn current_919_witness_consumes_sanitized_same_head_disposition() -> TestResult {
    let (recovered, current) = final_support::recover_919()?;
    let control = final_support::final_control_919(&recovered)?;
    let mut incomplete_change = control.clone();
    incomplete_change["post_cap_re_review"]["qualifying_change"]["finding_ids"] =
        serde_json::json!([final_support::native_919_remaining_finding]);
    let incomplete_change = final_support::canonical_third_predecessor(
        &incomplete_change,
        &current,
        &recovered,
    )
    .err()
    .ok_or("incomplete change unexpectedly passed the ordinary transition")?;
    let incomplete_diagnostic = incomplete_change.to_string();
    assert!(
        incomplete_diagnostic.contains("qualifying change evidence is not linked to the prior findings"),
        "unexpected incomplete-change diagnostic: {incomplete_diagnostic}"
    );
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
    assert_eq!(
        without_commit_oids(&state["nativeHistoryRecovery"]),
        without_commit_oids(&recovered["nativeHistoryRecovery"])
    );
    assert_eq!(
        state["nativeHistoryRecovery"]["binding"]["current_head"],
        state["headRefOid"]
    );
    assert_eq!(
        without_commit_oids(&state["reviewControl"]["terminal_review_history"]),
        without_commit_oids(&produced["terminal_review_history"])
    );
    assert!(state["reviewControl"].get("native_history_recovery").is_none());
    let handoff = final_support::validate_handoff_state(
        &state,
        final_support::native_919_pull_request,
    )?;
    assert!(handoff.status.success(), "#919 native-history handoff failed: {}", String::from_utf8_lossy(&handoff.stderr));
    Ok(())
}

fn without_commit_oids(value: &Value) -> Value {
    match value {
        Value::String(text) if text.len() == 40 && text.bytes().all(|byte| byte.is_ascii_hexdigit()) => {
            Value::String("<synthetic-commit>".into())
        }
        Value::Array(values) => Value::Array(values.iter().map(without_commit_oids).collect()),
        Value::Object(values) => Value::Object(
            values
                .iter()
                .map(|(key, value)| (key.clone(), without_commit_oids(value)))
                .collect(),
        ),
        value => value.clone(),
    }
}

#[test]
fn direct_handoff_rejects_unobservable_final_disposition() -> TestResult {
    let mut control = final_support::same_head_control(993);
    control["terminal_result"] = serde_json::json!("UNOBSERVABLE");
    control["terminal_review_history"][2]["terminal_result"] = serde_json::json!("UNOBSERVABLE");
    let previous = final_support::third_block_predecessor(&control);
    let producer_error = final_support::produce(&control, &previous)
        .err()
        .ok_or("UNOBSERVABLE producer unexpectedly succeeded")?;
    assert!(producer_error
        .to_string()
        .contains("review control final disposition requires the authentic third BLOCK"));
    let handoff = final_support::validate_handoff_with_state(
        &control,
        993,
        direct_state::SYNTHETIC_CURRENT_HEAD,
        |_| Ok(()),
    )?;
    assert!(!handoff.status.success());
    let diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&handoff.stdout),
        String::from_utf8_lossy(&handoff.stderr)
    );
    assert!(
        diagnostic.contains("review control final disposition requires the authentic third BLOCK"),
        "UNOBSERVABLE handoff was rejected for an unexpected reason: {diagnostic}"
    );
    Ok(())
}
