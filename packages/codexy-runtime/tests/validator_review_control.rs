use std::{fs, process::Command};

use crate::support::TestResult;
use serde_json::json;

#[test]
fn review_control_producer_writes_only_direct_state() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let input = temporary.path().join("input.json");
    let output = temporary.path().join("control.json");
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": {
                "schema": "codexy.review-control-state.v1",
                "profile": "strict",
                "reviewer": {"name": "codexy-sentinel"},
                "reviewed_head": "head",
                "terminal_result": "PASS",
                "unresolved_findings": [],
                "full_review_count": 1,
                "delta_review_count": 0,
            }
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

fn child_audit_evidence(review_record: &str) -> String {
    format!(
        "Lane ownership: child-owned\nSource thread id: parent-725\nNonterminal wait handoff: state fingerprint=fp-725; producer state=ci-queued; wake route=resume; ownership=retained; goal state=active; plan state=active; goal transition=none; return control=confirmed\n{review_record}\nTerminal parent handoff: event id=terminal-child|725|complete; issue/pr=#725 / PR #757; child task=child-725; parent task=parent-725; branch=eunsoogi/725-collapse-review-ceremony; worktree=/worktree; head=head; clean/index=clean; last proof=focused validator; current gate=parent review; preserved reservation/artifacts=worktree reserved; parent next action=inspect the PR; delivery=confirmed; task surface=codex task/thread\nGoal tool call: update_goal(status=\"complete\")\n"
    )
}

#[test]
fn lifecycle_audit_requires_explicit_terminal_fields() -> TestResult {
    let record = r#"{"reviewed_head":"head","profile":"strict","reviewer":{"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"},"state":"passed"}"#;
    let output = crate::support::validator_child_lane_ownership(&child_audit_evidence(record))?;
    assert!(!output.status.success(), "terminal records without direct evidence must block");
    Ok(())
}

#[test]
fn lifecycle_audit_recognizes_exact_pass_and_block() -> TestResult {
    for result in ["PASS", "BLOCK"] {
        let record = format!(
            r#"{{"reviewed_head":"head","profile":"strict","reviewer":{{"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"}},"terminal_result":"{result}","unresolved_findings":[],"full_review_count":1,"delta_review_count":0}}"#
        );
        let output = crate::support::validator_child_lane_ownership(&child_audit_evidence(&record))?;
        assert!(output.status.success(), "exact {result} must be a typed terminal: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}
