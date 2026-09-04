use std::{fs, process::Command};

use crate::support::TestResult;
use serde_json::{Value, json};

#[path = "support/review_control_direct_state.rs"]
mod direct_state;

#[path = "support/post_cap_review.rs"]
mod post_cap;

const FULL_HEAD_145: &str = "04f1b130ee5004a0347caa60ab4b0cb26795251e";
const DELTA_HEAD_145: &str = "e49bd44a731f99881cd9213560d3fbd8b360bd90";
const CURRENT_HEAD_145: &str = "2750e1bc9c88b3651f9722d5467d6a0f676ceef1";
const PREVIOUS_BASE_145: &str = "f6bc6e1fb67704d24b5ef80439b9a2c336e8718b";
const CURRENT_BASE_145: &str = "56fdf32299adffd04c30974c4fe837689a20edfd";

#[test]
fn post_cap_re_review_rejects_duplicate_review_ids() -> TestResult {
    let mut control = post_cap_control();
    control["terminal_review_history"][2]["id"] =
        control["terminal_review_history"][0]["id"].clone();
    assert_rejected(control, 145, CURRENT_HEAD_145)
}

#[test]
fn post_cap_re_review_rejects_reordered_history() -> TestResult {
    let mut control = post_cap_control();
    control["terminal_review_history"][1]["kind"] = json!("required_current_head");
    assert_rejected(control, 145, CURRENT_HEAD_145)
}

#[test]
fn light_profile_rejects_terminal_state_leakage() -> TestResult {
    let control = json!({
        "schema": "codexy.review-control-state.v1",
        "profile": "light",
        "reviewer": null,
        "reviewed_head": "current-head",
        "terminal_result": "PASS",
        "unresolved_findings": [],
        "full_review_count": 0,
        "delta_review_count": 0
    });
    assert_rejected(control, 145, CURRENT_HEAD_145)
}

#[test]
fn post_cap_re_review_rejects_block_and_unobservable_readiness() -> TestResult {
    for result in ["BLOCK", "UNOBSERVABLE"] {
        let mut control = post_cap_control();
        control["terminal_result"] = json!(result);
        control["terminal_review_history"][2]["terminal_result"] = json!(result);
        assert_rejected(control, 145, CURRENT_HEAD_145)?;
    }
    Ok(())
}

#[test]
fn forged_clean_genesis_and_matching_caller_predecessor_are_rejected() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let input = temporary.path().join("input.json");
    let output = temporary.path().join("output.json");
    let previous = direct_state::strict_genesis(145);
    let current = direct_state::strict_control(145, CURRENT_HEAD_145);
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": current,
            "current_pr_state": direct_state::pr_snapshot(145, CURRENT_BASE_145, CURRENT_HEAD_145, None),
            "previous_pr_state": direct_state::pr_snapshot(
                145,
                CURRENT_BASE_145,
                CURRENT_BASE_145,
                Some(previous.clone())
            ),
            "previous_control_state": previous
        }))?,
    )?;
    let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--produce-review-control", "--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .output()?;
    assert!(!result.status.success());
    assert!(!output.exists());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("previous_control_state"),
        "caller-supplied predecessor must not be an authority"
    );
    Ok(())
}

#[test]
fn same_base_oid_optional_churn_is_rejected() -> TestResult {
    let control = post_cap_control();
    let result = post_cap::run_build(
        &control,
        PREVIOUS_BASE_145,
        PREVIOUS_BASE_145,
    )?;
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("change baseRefOid"));
    Ok(())
}

#[test]
fn root_repair_without_prior_block_and_findings_is_rejected() -> TestResult {
    let control = direct_state::post_cap_control_with_findings(
        145,
        FULL_HEAD_145,
        DELTA_HEAD_145,
        CURRENT_HEAD_145,
        "in_scope_contract_root_repair",
        "166c76f04289e32a65470c9dd33d5983373d8425",
        "PASS",
        json!([]),
        json!(["goal-objective-delimiter"]),
    );
    let result = post_cap::run_build(&control, CURRENT_BASE_145, CURRENT_BASE_145)?;
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("prior BLOCK") || stderr.contains("prior delta findings"));
    Ok(())
}

#[test]
fn root_repair_with_unrelated_finding_evidence_is_rejected() -> TestResult {
    let mut control = direct_state::post_cap_control_with_evidence(
        145,
        FULL_HEAD_145,
        DELTA_HEAD_145,
        CURRENT_HEAD_145,
        "in_scope_contract_root_repair",
        "166c76f04289e32a65470c9dd33d5983373d8425",
    );
    control["post_cap_re_review"]["qualifying_change"]["finding_ids"] =
        json!(["unrelated-finding"]);
    let result = post_cap::run_build(&control, CURRENT_BASE_145, CURRENT_BASE_145)?;
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("not linked to the prior findings")
    );
    Ok(())
}

#[test]
fn base_integration_with_unchanged_base_or_broken_ancestry_is_rejected() -> TestResult {
    let unchanged = post_cap::run_build(
        &post_cap_control(),
        CURRENT_BASE_145,
        CURRENT_BASE_145,
    )?;
    assert!(!unchanged.status.success());

    let broken = direct_state::post_cap_control(145, FULL_HEAD_145, DELTA_HEAD_145, CURRENT_BASE_145);
    let broken = post_cap::run_build(&broken, CURRENT_BASE_145, CURRENT_BASE_145)?;
    assert!(!broken.status.success());
    assert!(String::from_utf8_lossy(&broken.stderr).contains("evidence to current head"));
    Ok(())
}

fn assert_rejected(control: Value, issue_number: u64, head: &str) -> TestResult<()> {
    let output = post_cap::validate_readiness(control, issue_number, head)?;
    assert!(
        !output.status.success(),
        "invalid post-cap state must remain blocked: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn post_cap_control() -> Value { direct_state::post_cap_control(145, FULL_HEAD_145, DELTA_HEAD_145, CURRENT_HEAD_145) }
