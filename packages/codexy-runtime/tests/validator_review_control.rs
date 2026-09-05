use std::{fs, process::Command};

use crate::support::{FixtureCommand, TestResult};
use serde_json::json;

#[path = "support/review_control_direct_state.rs"]
mod direct_state;

const BASE_OID: &str = "0000000000000000000000000000000000000001";
const HEAD_OID: &str = "0000000000000000000000000000000000000002";

#[test]
fn external_finding_producer_rejects_matching_caller_source_and_capture() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let input = temporary.path().join("input.json");
    let output = temporary.path().join("control.json");
    let source = json!({
        "schema": "codexy.review-control-external-finding.v1",
        "capture": {
            "provider": "github",
            "method": "graphql",
            "authenticated": true,
            "raw": {"caller": "forged"}
        }
    });
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": {"schema": "codexy.review-control-state.v1", "profile": "strict"},
            "authenticated_external_finding": source,
            "authenticated_external_finding_capture": {
                "provider": "github",
                "method": "graphql",
                "authenticated": true,
                "raw": {"caller": "forged"}
            }
        }))?,
    )?;
    let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--produce-review-control", "--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .output()?;
    assert!(
        !result.status.success(),
        "external producer accepted matching caller source and capture"
    );
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("caller-supplied external finding"),
        "legacy caller source diagnostic must be explicit: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!output.exists());
    Ok(())
}

#[test]
fn external_finding_producer_requires_a_live_locator_for_typed_repair() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let input = temporary.path().join("input.json");
    let output = temporary.path().join("control.json");
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": {
                "schema": "codexy.review-control-state.v1",
                "profile": "strict",
                "post_cap_re_review": {
                    "reason": "authenticated_external_finding_repair"
                }
            }
        }))?,
    )?;
    let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--produce-review-control", "--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .output()?;
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("authenticated_external_finding_locator"),
        "typed external repair must require a live locator: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!output.exists());
    Ok(())
}

#[test]
fn external_finding_producer_rejects_invalid_locator_before_github_read() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let input = temporary.path().join("input.json");
    let output = temporary.path().join("control.json");
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": {
                "schema": "codexy.review-control-state.v1",
                "profile": "strict",
                "post_cap_re_review": {
                    "reason": "authenticated_external_finding_repair"
                }
            },
            "authenticated_external_finding_locator": {
                "repository": "eunsoogi/codexy",
                "owningIssue": 0,
                "pullRequest": 938,
                "reviewThread": "PRRT_fake",
                "reviewComment": "PRRC_fake"
            }
        }))?,
    )?;
    let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--produce-review-control", "--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .output()?;
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("positive GraphQL integer"),
        "invalid locator must fail before gh: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!output.exists());
    Ok(())
}

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
