use std::fs;

use serde_json::{Value, json};

use crate::support::{FixtureCommand, TestResult};

#[path = "review_control_direct_state.rs"]
mod direct_state;

pub(crate) fn validate_readiness(
    control: Value,
    issue_number: u64,
    head: &str,
) -> TestResult<std::process::Output> {
    let temporary = tempfile::tempdir()?;
    let handoff = temporary.path().join("handoff.md");
    let state = temporary.path().join("state.json");
    fs::write(&handoff, "PASS on the exact current head.\n")?;
    fs::write(
        &state,
        serde_json::to_vec(&json!({
            "number": issue_number,
            "state": "OPEN",
            "isDraft": true,
            "mergeStateStatus": "CLEAN",
            "headRefOid": head,
            "reviewProfile": control["profile"].clone(),
            "reviewControl": control
        }))?,
    )?;
    Ok(crate::support::validator_completion_handoff_files(
        &handoff, &state,
    )?)
}

pub(crate) fn build_pr_state(
    control: &Value,
    previous_base: &str,
    current_base: &str,
) -> TestResult<Value> {
    let temporary = tempfile::tempdir()?;
    let output = temporary.path().join("pr-state.json");
    let (current, control_path, previous) = write_review_inputs(
        temporary.path(),
        control,
        previous_base,
        current_base,
    )?;
    let result = invoke_build(&current, &control_path, &previous, &output)?;
    assert!(
        result.status.success(),
        "build-pr-state must accept the typed post-cap state: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(serde_json::from_slice(&fs::read(output)?)?)
}

pub(crate) fn run_build(
    control: &Value,
    previous_base: &str,
    current_base: &str,
) -> TestResult<std::process::Output> {
    let temporary = tempfile::tempdir()?;
    let output = temporary.path().join("pr-state.json");
    let (current, control_path, previous) = write_review_inputs(
        temporary.path(),
        control,
        previous_base,
        current_base,
    )?;
    Ok(invoke_build(&current, &control_path, &previous, &output)?)
}

fn write_review_inputs(
    root: &std::path::Path,
    control: &Value,
    previous_base: &str,
    current_base: &str,
) -> TestResult<(
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
)> {
    let issue_number = control["issue_number"].as_u64().ok_or("issue number")?;
    let previous = direct_state::post_cap_prior(control);
    let previous_head = previous["reviewed_head"]
        .as_str()
        .ok_or("prior head")?
        .to_owned();
    let current_head = control["reviewed_head"].as_str().ok_or("current head")?;
    let current = root.join("current-pr-state.json");
    let control_path = root.join("review-control.json");
    let previous_path = root.join("previous-pr-state.json");
    fs::write(
        &current,
        serde_json::to_vec(&direct_state::pr_snapshot(
            issue_number,
            current_base,
            current_head,
            None,
        ))?,
    )?;
    fs::write(&control_path, serde_json::to_vec(control)?)?;
    fs::write(
        &previous_path,
        serde_json::to_vec(&direct_state::pr_snapshot(
            issue_number,
            previous_base,
            &previous_head,
            Some(previous),
        ))?,
    )?;
    Ok((current, control_path, previous_path))
}

fn invoke_build(
    current: &std::path::Path,
    control: &std::path::Path,
    previous: &std::path::Path,
    output: &std::path::Path,
) -> TestResult<std::process::Output> {
    let mut command = FixtureCommand::new(
        codexy_runtime::paths::repository_root().join("scripts/build-pr-state"),
    );
    command
        .arg("--base-pr-state-file")
        .arg_path(current)
        .arg("--review-control-state-file")
        .arg_path(control)
        .arg("--previous-pr-state-file")
        .arg_path(previous)
        .arg("--output")
        .arg_path(output)
        .env_path(
            "CODEXY_REVIEW_CONTROL_BIN",
            env!("CARGO_BIN_EXE_codexy-review-control"),
        );
    Ok(command.output()?)
}
