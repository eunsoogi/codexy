use std::{fs, path::Path};

use serde_json::{json, Value};

use crate::support::{FixtureCommand, TestResult};

#[test]
fn direct_review_control_accepts_state_without_ceremony() -> TestResult {
    let state = capture(direct_control())?;
    let control = &state["reviewControl"];
    assert_eq!(control["profile"], "strict");
    assert_eq!(control["reviewer"]["name"], "codexy-sentinel");
    assert_eq!(control["reviewed_head"], "head");
    assert_eq!(control["terminal_result"], "PASS");
    assert_eq!(control["unresolved_findings"], json!([]));
    assert_eq!(control["full_review_count"], 1);
    assert!(control.get("ledger").is_none());
    assert!(control.get("packet").is_none());
    Ok(())
}

#[test]
fn direct_review_control_rejects_the_closed_negative_cases() -> TestResult {
    for mutate in [
        |control: &mut Value| {
            control["profile"] = json!("standard");
        },
        |control: &mut Value| {
            control["reviewer"]["name"] = json!("codexy-inspector");
        },
        |control: &mut Value| {
            control["reviewed_head"] = json!("stale");
        },
        |control: &mut Value| {
            control["terminal_result"] = json!("SUCCESS");
        },
        |control: &mut Value| {
            control["full_review_count"] = json!(2);
        },
    ] {
        let mut control = direct_control();
        mutate(&mut control);
        assert!(
            !capture_output(control)?.status.success(),
            "direct-state negative case must remain blocked"
        );
    }
    Ok(())
}

#[test]
fn direct_review_control_blocks_terminal_failures_and_findings() -> TestResult {
    for (result, findings) in [
        ("BLOCK", json!([])),
        ("UNOBSERVABLE", json!([])),
        ("PASS", json!(["f-1"])),
    ] {
        let mut control = direct_control();
        control["terminal_result"] = json!(result);
        control["unresolved_findings"] = findings;
        let output = validate_readiness(control)?;
        assert!(
            !output.status.success(),
            "invalid readiness state must block"
        );
    }
    Ok(())
}

#[test]
fn direct_review_control_ignores_legacy_fields_and_prose_shape() -> TestResult {
    let mut control = direct_control();
    control["legacy_state"] = json!({"schema":"ignored","events":[{"state":"invalid"}]});
    let output = validate_readiness(control)?;
    assert!(
        output.status.success(),
        "direct facts must decide readiness: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn direct_control() -> Value {
    json!({
        "schema": "codexy.review-control-state.v1",
        "profile": "strict",
        "reviewer": {
            "name": "codexy-sentinel",
            "model": "gpt-5.6-sol",
            "reasoning_effort": "xhigh"
        },
        "reviewed_head": "head",
        "terminal_result": "PASS",
        "unresolved_findings": [],
        "full_review_count": 1,
        "delta_review_count": 0
    })
}

fn capture(control: Value) -> TestResult<Value> {
    let temp = tempfile::tempdir()?;
    let (base, control_path, output) = state_files(temp.path(), &control)?;
    let result = run_capture(&base, &control_path, &output)?;
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(serde_json::from_slice(&fs::read(output)?)?)
}

fn capture_output(control: Value) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let (base, control_path, output) = state_files(temp.path(), &control)?;
    Ok(run_capture(&base, &control_path, &output)?)
}

fn validate_readiness(control: Value) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let handoff = temp.path().join("handoff.md");
    let state = temp.path().join("state.json");
    fs::write(&handoff, "임의의 prose와 순서입니다.\n")?;
    fs::write(
        &state,
        serde_json::to_vec(&json!({
            "number": 725,
            "state": "OPEN",
            "isDraft": true,
            "mergeStateStatus": "CLEAN",
            "headRefOid": "head",
            "reviewControl": control
        }))?,
    )?;
    Ok(crate::support::validator_completion_handoff_files(
        &handoff, &state,
    )?)
}

fn state_files(
    root: &Path,
    control: &Value,
) -> TestResult<(std::path::PathBuf, std::path::PathBuf, std::path::PathBuf)> {
    let base = root.join("base.json");
    let control_path = root.join("review-control.json");
    let output = root.join("pr-state.json");
    fs::write(
        &base,
        serde_json::to_vec(&json!({
            "number": 725,
            "headRefOid": "head",
            "reviewDecision": "APPROVED"
        }))?,
    )?;
    fs::write(&control_path, serde_json::to_vec(control)?)?;
    Ok((base, control_path, output))
}

fn run_capture(base: &Path, control: &Path, output: &Path) -> TestResult<std::process::Output> {
    let mut command = FixtureCommand::new(
        codexy_runtime::paths::repository_root().join("scripts/build-pr-state"),
    );
    command
        .arg("--base-pr-state-file")
        .arg_path(base)
        .arg("--review-control-state-file")
        .arg_path(control)
        .arg("--output")
        .arg_path(output)
        .env_path(
            "CODEXY_REVIEW_CONTROL_BIN",
            env!("CARGO_BIN_EXE_codexy-review-control"),
        );
    Ok(command.output()?)
}
