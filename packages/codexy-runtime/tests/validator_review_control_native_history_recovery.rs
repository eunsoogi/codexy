use std::{
    fs,
    process::{Command, Output},
};

use serde_json::{Value, json};

use crate::support::TestResult;

#[path = "validator_review_control_native_history_recovery/capture.rs"]
mod capture;
#[path = "validator_review_control_native_history/fixtures.rs"]
mod fixtures;
#[path = "support/review_control_import.rs"]
mod import_support;

use import_support::{git_sha, run_build, stderr};

#[test]
fn recovery_rejects_unbound_timing_and_existing_history() -> TestResult {
    let mut unbound = fixtures::recovery_snapshot();
    let _ = unbound
        .as_object_mut()
        .ok_or("current snapshot")?
        .remove("createdAtEpoch");
    let (result, _) = run_recovery(&unbound, &fixtures::request())?;
    assert!(!result.status.success());
    assert!(stderr(&result).contains("proved_post_pr"));

    let mut existing = fixtures::recovery_snapshot();
    existing["reviewControl"] = json!({"schema": "codexy.review-control-state.v1"});
    let (result, _) = run_recovery(&existing, &fixtures::request())?;
    assert!(!result.status.success());
    assert!(stderr(&result).contains("replace existing reviewControl"));
    Ok(())
}

#[test]
fn cli_recovery_can_feed_next_legitimate_transition() -> TestResult {
    let (recovered, current, delta_head, current_head) = real_recovery_fixture()?;
    let control = fixtures::next_control(&recovered, &delta_head, &current_head)?;
    let (result, state) = run_build(&current, &control, &recovered)?;
    assert!(
        result.status.success(),
        "recovered predecessor transition failed: {}",
        stderr(&result)
    );
    let state = state.ok_or("successful transition must write state")?;
    assert_eq!(
        state["nativeHistoryRecovery"],
        recovered["nativeHistoryRecovery"]
    );
    assert_eq!(state["reviewControl"]["terminal_review_count"], json!(3));
    assert_eq!(state["reviewControl"]["full_review_count"], json!(1));
    assert_eq!(state["reviewControl"]["delta_review_count"], json!(1));
    assert_eq!(
        state["reviewControl"]["terminal_review_history"]
            .as_array()
            .map(Vec::len),
        Some(3)
    );
    for index in 0..2 {
        assert_eq!(
            state["reviewControl"]["terminal_review_history"][index],
            recovered["reviewControl"]["terminal_review_history"][index]
        );
    }
    assert_eq!(
        state["reviewControl"]["terminal_review_history"][0]["reviewer"]["model"],
        "model.continuation"
    );
    assert_eq!(
        state["reviewControl"]["terminal_review_history"][1]["source_reviewer"]["model"],
        "model.initial"
    );
    assert!(
        state["reviewControl"]
            .get("native_history_recovery")
            .is_none()
    );
    assert!(
        state["reviewControl"]
            .get("native_history_provenance")
            .is_some()
    );
    Ok(())
}

#[test]
fn recovered_control_is_not_admission_input() -> TestResult {
    let current = fixtures::recovery_snapshot();
    let (_, state) = run_recovery(&current, &fixtures::request())?;
    let mut state = state.ok_or("recovery state")?;
    state["state"] = json!("OPEN");
    state["isDraft"] = json!(false);
    state["mergeStateStatus"] = json!("CLEAN");
    let result = crate::support::validator_completion_handoff(
        "No blocked state while CI is pending.\n",
        &serde_json::to_string(&state)?,
    )?;
    assert!(!result.status.success());
    assert!(stderr(&result).contains("not eligible for current PR admission"));
    Ok(())
}

#[test]
fn recovered_provenance_rejects_forged_source_event_and_count() -> TestResult {
    let (recovered, current, delta_head, current_head) = real_recovery_fixture()?;

    let mut forged = recovered.clone();
    forged["nativeHistoryRecovery"]["events"][0]["reviewer"]["model"] = json!("model.forged");
    forged["reviewControl"]["terminal_review_history"][0]["reviewer"]["model"] =
        json!("model.forged");
    forged["reviewControl"]["terminal_review_history"][0]["source_reviewer"]["model"] =
        json!("model.forged");
    assert_eq!(
        forged["nativeHistoryRecovery"]["source"],
        recovered["nativeHistoryRecovery"]["source"]
    );
    let control = fixtures::next_control(&forged, &delta_head, &current_head)?;
    let (result, _) = run_build(&current, &control, &forged)?;
    assert!(!result.status.success());
    assert!(stderr(&result).contains("does not match preserved source"));

    let mut changed_current = current.clone();
    changed_current["nativeHistoryRecovery"]["source"]["reviewer"]["pages"][0]["page"]["limit"] =
        json!(11);
    let control = fixtures::next_control(&recovered, &delta_head, &current_head)?;
    let (result, _) = run_build(&changed_current, &control, &recovered)?;
    assert!(!result.status.success());

    for label in [
        "source reviewer",
        "event marker",
        "event count",
        "owning issue",
    ] {
        let mut candidate = recovered.clone();
        match label {
            "source reviewer" => {
                candidate["reviewControl"]["terminal_review_history"][0]["source_reviewer"]["model"] =
                    json!("model.forged");
            }
            "event marker" => {
                candidate["reviewControl"]["native_history_recovery"]["event_ids"][0] =
                    json!("forged-event");
            }
            "event count" => {
                candidate["reviewControl"]["terminal_review_count"] = json!(1);
            }
            "owning issue" => {
                candidate["reviewControl"]["issue_number"] = json!(18);
            }
            _ => unreachable!(),
        }
        let control = fixtures::next_control(&candidate, &delta_head, &current_head)?;
        let (result, _) = run_build(&current, &control, &candidate)?;
        assert!(!result.status.success(), "forged {label} must be rejected");
    }
    Ok(())
}

fn real_recovery_fixture() -> TestResult<(Value, Value, String, String)> {
    let full_head = git_sha("HEAD^^^")?;
    let delta_head = git_sha("HEAD^^")?;
    let recovery_head = git_sha("HEAD^")?;
    let current_head = git_sha("HEAD")?;
    let mut input = fixtures::read_thread_request();
    input["reviewer"]["pages"][1]["turns"][0]["items"][0]["review"]["reviewedHead"] =
        json!(full_head);
    input["reviewer"]["pages"][0]["turns"][0]["items"][0]["review"]["reviewedHead"] =
        json!(delta_head);
    let recovery_current = fixtures::snapshot_with_heads(50, &full_head, &recovery_head);
    let (result, state) = run_recovery(&recovery_current, &input)?;
    assert!(
        result.status.success(),
        "real-head recovery failed: {}",
        stderr(&result)
    );
    let recovered = state.ok_or("successful recovery must write state")?;
    let mut current = recovered.clone();
    let current_map = current.as_object_mut().ok_or("current snapshot")?;
    current_map.remove("reviewControl");
    current_map.insert("baseRefOid".into(), json!(recovery_head));
    current_map.insert("headRefOid".into(), json!(current_head.clone()));
    Ok((recovered, current, delta_head, current_head))
}

fn run_recovery(current: &Value, input: &Value) -> TestResult<(Output, Option<Value>)> {
    let temporary = tempfile::tempdir()?;
    let current_path = temporary.path().join("current.json");
    let input_path = temporary.path().join("native-history.json");
    let output_path = temporary.path().join("recovered.json");
    fs::write(&current_path, serde_json::to_vec(current)?)?;
    fs::write(&input_path, serde_json::to_vec(input)?)?;
    let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--recover-native-review-history", "--current-pr-state-file"])
        .arg(&current_path)
        .args(["--input"])
        .arg(&input_path)
        .args(["--output"])
        .arg(&output_path)
        .output()?;
    let state = result
        .status
        .success()
        .then(|| fs::read(&output_path))
        .transpose()?
        .map(|bytes| serde_json::from_slice(&bytes))
        .transpose()?;
    Ok((result, state))
}
