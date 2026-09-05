use std::{
    fs,
    process::{Command, Output},
};

use crate::support::TestResult;
use serde_json::{Value, json};

const REPOSITORY: &str = "eunsoogi/codexy";
const ISSUE: u64 = 946;
const PR: u64 = 942;

pub(crate) fn snapshot(head: &str, base: &str, control: Option<Value>) -> Value {
    let mut value = json!({
        "repository": REPOSITORY,
        "number": PR,
        "baseRefName": "main",
        "baseRefOid": base,
        "headRefOid": head,
        "url": format!("https://github.com/{REPOSITORY}/pull/{PR}"),
        "capture": {"provider": "github", "method": "graphql", "authenticated": true,
            "owningIssue": {"repository": REPOSITORY, "number": ISSUE,
                "url": format!("https://github.com/{REPOSITORY}/issues/{ISSUE}"),
                "association": "owner-assignment"}}
    });
    if let Some(control) = control {
        value["reviewControl"] = control;
    }
    value
}

pub(crate) fn envelope(events: Vec<Value>) -> Value {
    json!({"schema": "codexy.review-control-pre-pr-history.v1",
        "source": {"provider": "codex_app", "method": "rollout_jsonl", "authenticated": true, "host_id": "local"},
        "issue": {"repository": REPOSITORY, "number": ISSUE,
            "url": format!("https://github.com/{REPOSITORY}/issues/{ISSUE}")},
        "profile": "standard", "complete": true,
        "terminal_event_count": events.len(), "events": events})
}

pub(crate) fn event(id: &str, kind: &str, head: &str, turn: &str, ordinal: u64) -> Value {
    event_with_reviewer(
        id,
        kind,
        head,
        turn,
        ordinal,
        json!({"name": "codexy-inspector", "model": "gpt-5.6-sol", "reasoning_effort": "medium"}),
    )
}

pub(crate) fn legacy_event(id: &str, kind: &str, head: &str, turn: &str, ordinal: u64) -> Value {
    event_with_reviewer(
        id,
        kind,
        head,
        turn,
        ordinal,
        json!({"name": "codexy-inspector", "model": "gpt-5.6-terra", "reasoning_effort": "max"}),
    )
}

fn event_with_reviewer(
    id: &str,
    kind: &str,
    head: &str,
    turn: &str,
    ordinal: u64,
    reviewer: Value,
) -> Value {
    json!({"sequence": if kind == "full" { 0 } else { 1 }, "id": id,
        "thread_id": "thread-902", "turn_id": turn, "ordinal": ordinal,
        "turn_status": "completed", "item_type": "AgentMessage", "phase": "final_answer",
        "reviewer": reviewer, "kind": kind, "reviewed_head": head,
        "terminal_result": "PASS", "unresolved_findings": []})
}

pub(crate) fn light_control() -> Value {
    json!({"schema": "codexy.review-control-state.v1", "profile": "light"})
}

pub(crate) fn history_event(id: &str, kind: &str, head: &str) -> Value {
    json!({"id": id, "kind": kind,
        "reviewer": {"name": "codexy-inspector", "model": "gpt-5.6-sol", "reasoning_effort": "medium"},
        "reviewed_head": head, "terminal_result": "PASS", "unresolved_findings": []})
}

pub(crate) fn run_import(current: &Value, envelope: &Value) -> TestResult<(Output, Option<Value>)> {
    let temporary = tempfile::tempdir()?;
    let current_path = temporary.path().join("current.json");
    let input_path = temporary.path().join("history.json");
    let output_path = temporary.path().join("state.json");
    fs::write(&current_path, serde_json::to_vec(current)?)?;
    fs::write(&input_path, serde_json::to_vec(envelope)?)?;
    let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--import-pre-pr-history", "--current-pr-state-file"])
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

pub(crate) fn run_build(
    current: &Value,
    control: &Value,
    previous: &Value,
) -> TestResult<(Output, Option<Value>)> {
    let temporary = tempfile::tempdir()?;
    let paths = [
        "current.json",
        "control.json",
        "previous.json",
        "state.json",
    ]
    .map(|name| temporary.path().join(name));
    fs::write(&paths[0], serde_json::to_vec(current)?)?;
    fs::write(&paths[1], serde_json::to_vec(control)?)?;
    fs::write(&paths[2], serde_json::to_vec(previous)?)?;
    let mut command = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"));
    command
        .args(["--build-pr-state", "--base-pr-state-file"])
        .arg(&paths[0])
        .args(["--review-control-state-file"])
        .arg(&paths[1])
        .args(["--previous-pr-state-file"])
        .arg(&paths[2])
        .args(["--output"])
        .arg(&paths[3]);
    let result = command.output()?;
    let state = result
        .status
        .success()
        .then(|| fs::read(&paths[3]))
        .transpose()?
        .map(|bytes| serde_json::from_slice(&bytes))
        .transpose()?;
    Ok((result, state))
}

pub(crate) fn run_producer(
    current: &Value,
    control: &Value,
    previous: &Value,
) -> TestResult<(Output, Option<Value>)> {
    let temporary = tempfile::tempdir()?;
    let input_path = temporary.path().join("input.json");
    let output_path = temporary.path().join("control.json");
    let request = json!({
        "control_state": control,
        "current_pr_state": current,
        "previous_pr_state": previous,
    });
    fs::write(&input_path, serde_json::to_vec(&request)?)?;
    let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--produce-review-control", "--input"])
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

pub(crate) fn git_sha(reference: &str) -> TestResult<String> {
    let output = Command::new("git")
        .args(["rev-parse", reference])
        .current_dir(codexy_runtime::paths::repository_root())
        .output()?;
    assert!(
        output.status.success(),
        "git rev-parse failed: {}",
        stderr(&output)
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

pub(crate) fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
