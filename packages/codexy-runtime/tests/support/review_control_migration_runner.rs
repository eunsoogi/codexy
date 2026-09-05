use std::{fs, path::Path, process::Command};

use serde_json::{Value, json};

use crate::support::{FixtureCommand, TestResult};

pub(crate) const BASE_OID: &str = "0000000000000000000000000000000000000001";
pub(crate) const HEAD_OID: &str = "0000000000000000000000000000000000000002";
pub(crate) const MIGRATED_HEAD_OID: &str = "0000000000000000000000000000000000000003";

pub(crate) fn run_transition(
    previous_control: &Value,
    current_control: &Value,
) -> TestResult<(std::process::Output, Option<Value>)> {
    let temp = tempfile::tempdir()?;
    run_transition_with_repository(
        temp.path(),
        None,
        BASE_OID,
        BASE_OID,
        previous_control,
        current_control,
    )
}

pub(crate) fn run_transition_with_repository(
    root: &Path,
    repository: Option<&Path>,
    previous_base: &str,
    current_base: &str,
    previous_control: &Value,
    current_control: &Value,
) -> TestResult<(std::process::Output, Option<Value>)> {
    execute_transition(
        root,
        repository,
        previous_base,
        current_base,
        previous_control,
        current_control,
    )
}

pub(crate) fn run_producer(
    previous_control: &Value,
    current_control: &Value,
) -> TestResult<(std::process::Output, Option<Value>)> {
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("input.json");
    let output = temp.path().join("control.json");
    let previous_head = previous_control["reviewed_head"]
        .as_str()
        .ok_or("previous control reviewed_head")?;
    let current_head = current_control["reviewed_head"]
        .as_str()
        .ok_or("current control reviewed_head")?;
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": current_control,
            "current_pr_state": snapshot(725, BASE_OID, current_head, None),
            "previous_pr_state": snapshot(725, BASE_OID, previous_head, Some(previous_control))
        }))?,
    )?;
    let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--produce-review-control", "--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .output()?;
    let state = result
        .status
        .success()
        .then(|| fs::read(&output))
        .transpose()?
        .map(|bytes| serde_json::from_slice(&bytes))
        .transpose()?;
    Ok((result, state))
}

pub(crate) fn snapshot(
    issue_number: u64,
    base: &str,
    head: &str,
    control: Option<&Value>,
) -> Value {
    let mut snapshot = json!({
        "repository": "eunsoogi/codexy",
        "number": issue_number,
        "baseRefName": "main",
        "baseRefOid": base,
        "headRefOid": head,
        "url": format!("https://github.com/eunsoogi/codexy/pull/{issue_number}"),
        "capture": {
            "provider": "github",
            "method": "graphql",
            "authenticated": true,
            "owningIssue": {
                "repository": "eunsoogi/codexy",
                "number": issue_number,
                "url": format!("https://github.com/eunsoogi/codexy/issues/{issue_number}"),
                "association": "owner-assignment"
            }
        }
    });
    if let Some(control) = control {
        snapshot["reviewControl"] = control.clone();
    }
    snapshot
}

fn execute_transition(
    root: &Path,
    repository: Option<&Path>,
    previous_base: &str,
    current_base: &str,
    previous_control: &Value,
    current_control: &Value,
) -> TestResult<(std::process::Output, Option<Value>)> {
    let current = root.join("current-pr-state.json");
    let control = root.join("review-control.json");
    let previous = root.join("previous-pr-state.json");
    let output = root.join("pr-state.json");
    let previous_head = previous_control["reviewed_head"]
        .as_str()
        .ok_or("previous control reviewed_head")?;
    let current_head = current_control["reviewed_head"]
        .as_str()
        .ok_or("current control reviewed_head")?;
    fs::write(&current, serde_json::to_vec(&snapshot(725, current_base, current_head, None))?)?;
    fs::write(&control, serde_json::to_vec(current_control)?)?;
    fs::write(
        &previous,
        serde_json::to_vec(&snapshot(
            725,
            previous_base,
            previous_head,
            Some(previous_control),
        ))?,
    )?;
    let mut command = FixtureCommand::new(
        codexy_runtime::paths::repository_root().join("scripts/build-pr-state"),
    );
    if let Some(repository) = repository {
        command.arg("--repository-root").arg_path(repository);
    }
    command
        .arg("--base-pr-state-file")
        .arg_path(&current)
        .arg("--review-control-state-file")
        .arg_path(&control)
        .arg("--previous-pr-state-file")
        .arg_path(&previous)
        .arg("--output")
        .arg_path(&output)
        .env_path(
            "CODEXY_REVIEW_CONTROL_BIN",
            env!("CARGO_BIN_EXE_codexy-review-control"),
        );
    let result = command.output()?;
    let state = result
        .status
        .success()
        .then(|| fs::read(&output))
        .transpose()?
        .map(|bytes| serde_json::from_slice(&bytes))
        .transpose()?;
    Ok((result, state))
}
