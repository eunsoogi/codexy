use std::{fs, process::Command};

use crate::support::{FixtureCommand, TestResult};
use serde_json::json;

#[path = "support/review_control_direct_state.rs"]
mod direct_state;
#[path = "validator_review_control/connector.rs"]
mod connector;
#[path = "validator_review_control/retired.rs"]
mod retired;

const BASE_OID: &str = "0000000000000000000000000000000000000001";
const HEAD_OID: &str = "0000000000000000000000000000000000000002";

#[test]
fn review_control_producer_writes_only_direct_state() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let input = temporary.path().join("input.json");
    let output = temporary.path().join("control.json");
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": direct_state::strict_control(725, HEAD_OID),
            "current_pr_state": direct_state::pr_snapshot(725, BASE_OID, HEAD_OID, None),
            "previous_pr_state": direct_state::pr_snapshot(
                725,
                BASE_OID,
                BASE_OID,
                Some(direct_state::strict_genesis(725))
            )
        }))?,
    )?;
    let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--produce-review-control", "--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .status()?;
    assert!(result.success(), "direct producer must not require ceremony outputs");
    let produced: serde_json::Value = serde_json::from_slice(&fs::read(&output)?)?;
    assert_eq!(produced["schema"], "codexy.review-control-state.v1");
    assert!(produced.get("control_state").is_none());
    assert!(!produced.get("packet").is_some());
    assert!(!produced.get("ledger").is_some());
    Ok(())
}

#[test]
fn review_control_producer_accepts_light_controls_without_reviewer_state() -> TestResult {
    for control in [
        json!({"schema": "codexy.review-control-state.v1", "profile": "light"}),
        json!({"schema": "codexy.review-control-state.v1", "profile": "light", "reviewer": null}),
    ] {
        let temporary = tempfile::tempdir()?;
        let input = temporary.path().join("input.json");
        let output = temporary.path().join("control.json");
        let expected = control.clone();
        fs::write(
            &input,
            serde_json::to_vec(&json!({"control_state": control}))?,
        )?;
        let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
            .args(["--produce-review-control", "--input"])
            .arg(&input)
            .args(["--output"])
            .arg(&output)
            .status()?;
        assert!(result.success(), "light control must remain a valid route");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&fs::read(output)?)?,
            expected
        );
    }
    Ok(())
}

#[test]
fn producer_rejects_retired_shortcuts_in_a_direct_control() -> TestResult {
    for field in ["decision", "evidence", "ledger"] {
        let mut control = direct_state::strict_control(725, HEAD_OID);
        control[field] = serde_json::Value::Null;
        retired::assert_retired_request(control)?;
    }
    Ok(())
}

#[test]
fn build_pr_state_rejects_retired_previous_before_current_normalization() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let current = temporary.path().join("current.json");
    let control = temporary.path().join("control.json");
    let previous = temporary.path().join("previous.json");
    let output = temporary.path().join("output.json");
    let sentinel = b"preserve this output";
    fs::write(&current, b"{}")?;
    fs::write(
        &control,
        serde_json::to_vec(&direct_state::strict_control(725, HEAD_OID))?,
    )?;
    fs::write(
        &previous,
        br#"{"reviewControl":{"full_review_count":null}}"#,
    )?;
    fs::write(&output, sentinel)?;

    let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--build-pr-state", "--base-pr-state-file"])
        .arg(&current)
        .args(["--review-control-state-file"])
        .arg(&control)
        .args(["--previous-pr-state-file"])
        .arg(&previous)
        .args(["--output"])
        .arg(&output)
        .output()?;
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("legacy review-control processing is no longer supported")
    );
    assert_eq!(fs::read(&output)?, sentinel);
    Ok(())
}

#[test]
fn build_pr_state_skips_review_transition_for_light_controls() -> TestResult {
    for control in [
        json!({"schema": "codexy.review-control-state.v1", "profile": "light"}),
        json!({"schema": "codexy.review-control-state.v1", "profile": "light", "reviewer": null}),
    ] {
        let temporary = tempfile::tempdir()?;
        let current = temporary.path().join("current-pr-state.json");
        let control_path = temporary.path().join("review-control.json");
        let previous = temporary.path().join("previous-pr-state.json");
        let output = temporary.path().join("pr-state.json");
        fs::write(
            &current,
            serde_json::to_vec(&direct_state::pr_snapshot(725, BASE_OID, HEAD_OID, None))?,
        )?;
        fs::write(&control_path, serde_json::to_vec(&control)?)?;
        fs::write(
            &previous,
            serde_json::to_vec(&direct_state::pr_snapshot(725, BASE_OID, BASE_OID, None))?,
        )?;
        let mut command = FixtureCommand::new(
            codexy_runtime::paths::repository_root().join("scripts/build-pr-state"),
        );
        command
            .arg("--base-pr-state-file")
            .arg_path(&current)
            .arg("--review-control-state-file")
            .arg_path(&control_path)
            .arg("--previous-pr-state-file")
            .arg_path(&previous)
            .arg("--output")
            .arg_path(&output)
            .env_path(
                "CODEXY_REVIEW_CONTROL_BIN",
                env!("CARGO_BIN_EXE_codexy-review-control"),
            );
        let result = command.output()?;
        assert!(
            result.status.success(),
            "build-pr-state must preserve light controls: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let state: serde_json::Value = serde_json::from_slice(&fs::read(output)?)?;
        assert_eq!(state["reviewControl"], control);
    }
    Ok(())
}
