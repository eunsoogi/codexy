use std::{fs, process::Command};

use crate::support::TestResult;
use serde_json::json;

#[path = "support/review_control_direct_state.rs"]
mod direct_state;

#[path = "support/post_cap_review.rs"]
mod post_cap;

const FULL_HEAD_145: &str = direct_state::SYNTHETIC_FULL_HEAD;
const DELTA_HEAD_145: &str = direct_state::SYNTHETIC_DELTA_HEAD;
const CURRENT_HEAD_145: &str = direct_state::SYNTHETIC_CURRENT_HEAD;
const PREVIOUS_BASE_145: &str = direct_state::SYNTHETIC_BASE;
const CURRENT_BASE_145: &str = direct_state::SYNTHETIC_UPDATED_BASE;
const FULL_HEAD_873: &str = direct_state::SYNTHETIC_FULL_HEAD;
const DELTA_HEAD_873: &str = direct_state::SYNTHETIC_DELTA_HEAD;
const CURRENT_HEAD_873: &str = direct_state::SYNTHETIC_CURRENT_HEAD;

#[test]
fn post_cap_re_review_rejects_untyped_third_verdict() -> TestResult {
    let mut control = direct_state::post_cap_control(
        145,
        FULL_HEAD_145,
        DELTA_HEAD_145,
        CURRENT_HEAD_145,
    );
    control
        .as_object_mut()
        .expect("control object")
        .remove("post_cap_re_review");
    let output = post_cap::validate_readiness(control, 145, CURRENT_HEAD_145)?;
    assert!(
        !output.status.success(),
        "a third terminal verdict must carry a bounded post-cap reason"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("third terminal verdict"),
        "diagnostic must name the missing post-cap admission"
    );
    Ok(())
}

#[test]
fn post_cap_re_review_accepts_main_integration_after_full_and_delta() -> TestResult {
    let control = direct_state::post_cap_control(
        145,
        FULL_HEAD_145,
        DELTA_HEAD_145,
        CURRENT_HEAD_145,
    );
    let state = post_cap::build_pr_state(
        &control,
        PREVIOUS_BASE_145,
        CURRENT_BASE_145,
    )?;
    assert_eq!(state["reviewControl"]["terminal_review_count"], 3);
    assert_eq!(
        state["headRefOid"],
        state["reviewControl"]["reviewed_head"]
    );
    assert_eq!(
        state["reviewControl"]["post_cap_re_review"]["qualifying_change"]["to_head"],
        state["headRefOid"]
    );
    assert_eq!(state["baseRefOid"].as_str().map(str::len), Some(40));
    assert_eq!(
        state["reviewControl"]["post_cap_re_review"]["reason"],
        "mandatory_base_integration"
    );
    let output = post_cap::validate_readiness(control, 145, CURRENT_HEAD_145)?;
    assert!(
        output.status.success(),
        "required current-head re-review must pass readiness: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn post_cap_re_review_accepts_in_scope_contract_root_repair() -> TestResult {
    let control = direct_state::post_cap_control_with_evidence(
        873,
        FULL_HEAD_873,
        DELTA_HEAD_873,
        CURRENT_HEAD_873,
        "in_scope_contract_root_repair",
        direct_state::SYNTHETIC_REPAIR_EVIDENCE,
    );
    assert_eq!(
        control["post_cap_re_review"]["reason"],
        "in_scope_contract_root_repair"
    );
    let state = post_cap::build_pr_state(&control, PREVIOUS_BASE_145, PREVIOUS_BASE_145)?;
    assert_eq!(
        state["headRefOid"],
        state["reviewControl"]["reviewed_head"]
    );
    let output = post_cap::validate_readiness(control, 873, CURRENT_HEAD_873)?;
    assert!(
        output.status.success(),
        "in-scope contract/root repair must be an eligible reason: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn post_cap_re_review_rejects_churn_and_a_fourth_verdict() -> TestResult {
    let mut churn = direct_state::post_cap_control(
        145,
        FULL_HEAD_145,
        DELTA_HEAD_145,
        CURRENT_HEAD_145,
    );
    churn["post_cap_re_review"]["reason"] = json!("optional_churn");
    assert!(!post_cap::validate_readiness(churn, 145, CURRENT_HEAD_145)?.status.success());

    let mut fourth = direct_state::post_cap_control(
        145,
        FULL_HEAD_145,
        DELTA_HEAD_145,
        CURRENT_HEAD_145,
    );
    fourth["terminal_review_count"] = json!(4);
    fourth["terminal_review_history"]
        .as_array_mut()
        .expect("history array")
        .push(direct_state::review_event(
            "strict-fourth-1",
            "required_current_head",
            "fourth-head",
            "PASS",
        ));
    assert!(!post_cap::validate_readiness(fourth, 145, CURRENT_HEAD_145)?.status.success());
    Ok(())
}

#[test]
fn post_cap_re_review_rejects_duplicate_or_truncated_history() -> TestResult {
    let mut duplicate = direct_state::post_cap_control(
        145,
        FULL_HEAD_145,
        DELTA_HEAD_145,
        CURRENT_HEAD_145,
    );
    duplicate["post_cap_re_review"]["prior_reviewed_head"] = json!(CURRENT_HEAD_145);
    assert!(!post_cap::validate_readiness(duplicate, 145, CURRENT_HEAD_145)?.status.success());

    let mut repeated_head = direct_state::post_cap_control(
        145,
        FULL_HEAD_145,
        DELTA_HEAD_145,
        CURRENT_HEAD_145,
    );
    repeated_head["reviewed_head"] = json!(FULL_HEAD_145);
    repeated_head["terminal_review_history"][2]["reviewed_head"] = json!(FULL_HEAD_145);
    assert!(!post_cap::validate_readiness(repeated_head, 145, FULL_HEAD_145)?.status.success());

    let mut truncated = direct_state::post_cap_control(
        145,
        FULL_HEAD_145,
        DELTA_HEAD_145,
        CURRENT_HEAD_145,
    );
    truncated["terminal_review_history"]
        .as_array_mut()
        .expect("history array")
        .remove(0);
    assert!(!post_cap::validate_readiness(truncated, 145, CURRENT_HEAD_145)?.status.success());

    let mut reset = direct_state::post_cap_control(
        145,
        FULL_HEAD_145,
        DELTA_HEAD_145,
        CURRENT_HEAD_145,
    );
    reset["terminal_review_count"] = json!(2);
    reset["terminal_review_history"]
        .as_array_mut()
        .expect("history array")
        .pop();
    assert!(!post_cap::validate_readiness(reset, 145, CURRENT_HEAD_145)?.status.success());
    Ok(())
}

#[test]
fn post_cap_re_review_rejects_projection_and_issue_identity_drift() -> TestResult {
    let mut projection = direct_state::post_cap_control(
        145,
        FULL_HEAD_145,
        DELTA_HEAD_145,
        CURRENT_HEAD_145,
    );
    projection["terminal_review_history"][2]["terminal_result"] = json!("BLOCK");
    assert!(!post_cap::validate_readiness(projection, 145, CURRENT_HEAD_145)?.status.success());

    let mut wrong_issue = direct_state::post_cap_control(
        145,
        FULL_HEAD_145,
        DELTA_HEAD_145,
        CURRENT_HEAD_145,
    );
    wrong_issue["issue_number"] = json!(873);
    assert!(!post_cap::validate_readiness(wrong_issue, 145, CURRENT_HEAD_145)?.status.success());
    Ok(())
}

#[test]
fn post_cap_producer_rejects_unqualified_reason_without_writing_output() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let input = temporary.path().join("input.json");
    let output = temporary.path().join("control.json");
    let mut control = direct_state::post_cap_control(
        145,
        FULL_HEAD_145,
        DELTA_HEAD_145,
        CURRENT_HEAD_145,
    );
    control["post_cap_re_review"]["reason"] = json!("optional_churn");
    let previous = direct_state::post_cap_prior(&control);
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": control,
            "current_pr_state": direct_state::pr_snapshot(145, CURRENT_BASE_145, CURRENT_HEAD_145, None),
            "previous_pr_state": direct_state::pr_snapshot(145, PREVIOUS_BASE_145, DELTA_HEAD_145, Some(previous))
        }))?,
    )?;
    let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--produce-review-control", "--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .output()?;
    assert!(!result.status.success(), "producer must reject optional churn");
    assert!(!output.exists(), "rejected state must not write a producer output");
    assert!(String::from_utf8_lossy(&result.stderr).contains("post-cap reason"));
    Ok(())
}
