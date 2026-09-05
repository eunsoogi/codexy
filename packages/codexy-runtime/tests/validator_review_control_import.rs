use std::{fs, process::{Command, Output}};

use crate::support::TestResult;
use serde_json::{Value, json};

const REPOSITORY: &str = "eunsoogi/codexy";
const ISSUE: u64 = 946;
const PR: u64 = 942;

#[test]
fn import_preserves_current_pr_identity_and_host_receipt_refs() -> TestResult {
    let current_head = git_sha("HEAD")?;
    let reviewed_head = git_sha("HEAD^")?;
    let current = snapshot(&current_head, &reviewed_head, None);
    let envelope = envelope(vec![event("msg-full", "full", &reviewed_head, "turn-full", 147)]);
    let (result, state) = run_import(&current, &envelope)?;
    assert!(result.status.success(), "import failed: {}", stderr(&result));
    let state = state.expect("successful import must write state");
    assert_eq!(state["number"], json!(PR));
    assert_eq!(state["url"], format!("https://github.com/{REPOSITORY}/pull/{PR}"));
    assert_eq!(state["baseRefOid"], reviewed_head);
    assert_eq!(state["headRefOid"], current_head);
    assert_eq!(state["reviewControl"]["issue_number"], json!(ISSUE));
    assert_eq!(state["reviewControl"]["reviewed_head"], reviewed_head);
    assert_eq!(state["reviewControl"]["terminal_review_history"][0]["id"], "msg-full");
    assert_eq!(state["reviewControl"]["pre_pr_import"]["events"][0]["turn_id"], "turn-full");
    Ok(())
}

#[test]
fn import_rejects_incomplete_duplicate_and_reordered_receipts() -> TestResult {
    let current_head = git_sha("HEAD")?;
    let first = git_sha("HEAD^^")?;
    let second = git_sha("HEAD^")?;
    let current = snapshot(&current_head, &second, None);
    let full = event("msg-full", "full", &first, "turn-full", 10);
    let delta = event("msg-delta", "delta", &second, "turn-delta", 20);
    let valid = envelope(vec![full.clone(), delta.clone()]);
    let mut cases = Vec::new();
    let mut incomplete = valid.clone();
    incomplete["complete"] = json!(false);
    cases.push(("incomplete", incomplete));
    let mut duplicate = valid.clone();
    duplicate["events"][1]["id"] = json!("msg-full");
    cases.push(("duplicate", duplicate));
    let mut reordered = valid.clone();
    reordered["events"][0]["kind"] = json!("delta");
    cases.push(("reordered", reordered));
    for (label, candidate) in cases {
        let (result, _) = run_import(&current, &candidate)?;
        assert!(!result.status.success(), "{label} receipt must be rejected");
    }
    Ok(())
}

#[test]
fn import_rejects_wrong_issue_and_existing_history() -> TestResult {
    let current_head = git_sha("HEAD")?;
    let reviewed_head = git_sha("HEAD^")?;
    let current = snapshot(&current_head, &reviewed_head, None);
    let mut wrong_issue = envelope(vec![event("msg-full", "full", &reviewed_head, "turn-full", 1)]);
    wrong_issue["issue"]["number"] = json!(ISSUE - 1);
    wrong_issue["issue"]["url"] = json!(format!("https://github.com/{REPOSITORY}/issues/{}", ISSUE - 1));
    let (result, _) = run_import(&current, &wrong_issue)?;
    assert!(!result.status.success(), "wrong owning issue must be rejected");

    let existing = snapshot(&current_head, &reviewed_head, Some(json!({"history": []})));
    let (result, _) = run_import(&existing, &envelope(vec![event(
        "msg-full", "full", &reviewed_head, "turn-full", 1,
    )]))?;
    assert!(!result.status.success(), "genesis import must not replace existing history");
    Ok(())
}

#[test]
fn ordinary_transition_inherits_import_marker() -> TestResult {
    let current_head = git_sha("HEAD")?;
    let reviewed_head = git_sha("HEAD^")?;
    let imported = snapshot(&current_head, &reviewed_head, None);
    let (_, imported_state) = run_import(
        &imported,
        &envelope(vec![event("msg-full", "full", &reviewed_head, "turn-full", 1)]),
    )?;
    let imported_state = imported_state.expect("import state");
    let previous = imported_state;
    let current = snapshot(&current_head, &current_head, None);
    let mut control = previous["reviewControl"].clone();
    control["reviewed_head"] = json!(current_head);
    control["delta_review_count"] = json!(1);
    control["terminal_review_count"] = json!(2);
    control["terminal_review_history"] = json!([
        previous["reviewControl"]["terminal_review_history"][0].clone(),
        history_event("msg-delta", "delta", &current_head)
    ]);
    let (result, state) = run_build(&current, &control, &previous)?;
    assert!(result.status.success(), "transition failed: {}", stderr(&result));
    assert_eq!(state.expect("transition state")["reviewControl"]["pre_pr_import"], previous["reviewControl"]["pre_pr_import"]);
    let mut removed = control.clone();
    removed.as_object_mut().expect("control object").remove("pre_pr_import");
    let (result, _) = run_build(&current, &removed, &previous)?;
    assert!(!result.status.success(), "removed import marker must be rejected");
    let mut changed = control;
    changed["pre_pr_import"]["complete"] = json!(false);
    let (result, _) = run_build(&current, &changed, &previous)?;
    assert!(!result.status.success(), "changed import marker must be rejected");
    Ok(())
}

fn snapshot(head: &str, base: &str, control: Option<Value>) -> Value {
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

fn envelope(events: Vec<Value>) -> Value {
    json!({"schema": "codexy.review-control-pre-pr-history.v1",
        "source": {"provider": "codex_app", "method": "rollout_jsonl", "authenticated": true, "host_id": "local"},
        "issue": {"repository": REPOSITORY, "number": ISSUE,
            "url": format!("https://github.com/{REPOSITORY}/issues/{ISSUE}")},
        "profile": "standard", "complete": true,
        "terminal_event_count": events.len(), "events": events})
}

fn event(id: &str, kind: &str, head: &str, turn: &str, ordinal: u64) -> Value {
    json!({"sequence": if kind == "full" { 0 } else { 1 }, "id": id,
        "thread_id": "thread-902", "turn_id": turn, "ordinal": ordinal,
        "turn_status": "completed", "item_type": "AgentMessage", "phase": "final_answer",
        "reviewer": {"name": "codexy-inspector", "model": "gpt-5.6-sol", "reasoning_effort": "medium"},
        "kind": kind, "reviewed_head": head, "terminal_result": "PASS",
        "unresolved_findings": []})
}

fn history_event(id: &str, kind: &str, head: &str) -> Value {
    json!({"id": id, "kind": kind,
        "reviewer": {"name": "codexy-inspector", "model": "gpt-5.6-sol", "reasoning_effort": "medium"},
        "reviewed_head": head, "terminal_result": "PASS", "unresolved_findings": []})
}

fn run_import(current: &Value, envelope: &Value) -> TestResult<(Output, Option<Value>)> {
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
    let state = result.status.success().then(|| fs::read(&output_path)).transpose()?.map(|bytes| serde_json::from_slice(&bytes)).transpose()?;
    Ok((result, state))
}

fn run_build(current: &Value, control: &Value, previous: &Value) -> TestResult<(Output, Option<Value>)> {
    let temporary = tempfile::tempdir()?;
    let paths = ["current.json", "control.json", "previous.json", "state.json"].map(|name| temporary.path().join(name));
    fs::write(&paths[0], serde_json::to_vec(current)?)?;
    fs::write(&paths[1], serde_json::to_vec(control)?)?;
    fs::write(&paths[2], serde_json::to_vec(previous)?)?;
    let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--build-pr-state", "--base-pr-state-file"])
        .arg(&paths[0])
        .args(["--review-control-state-file"])
        .arg(&paths[1])
        .args(["--previous-pr-state-file"])
        .arg(&paths[2])
        .args(["--output"])
        .arg(&paths[3])
        .output()?;
    let state = result.status.success().then(|| fs::read(&paths[3])).transpose()?.map(|bytes| serde_json::from_slice(&bytes)).transpose()?;
    Ok((result, state))
}

fn git_sha(reference: &str) -> TestResult<String> {
    let output = Command::new("git").args(["rev-parse", reference]).current_dir(codexy_runtime::paths::repository_root()).output()?;
    assert!(output.status.success(), "git rev-parse failed: {}", stderr(&output));
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
