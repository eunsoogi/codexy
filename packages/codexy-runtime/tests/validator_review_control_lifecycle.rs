use crate::support::TestResult;

fn child_audit_evidence(review_record: &str) -> String {
    format!(
        "Lane ownership: child-owned\nSource thread id: parent-725\nNonterminal wait handoff: state fingerprint=fp-725; producer state=ci-queued; wake route=resume; ownership=retained; goal state=active; plan state=active; goal transition=none; return control=confirmed\n{review_record}\nTerminal parent handoff: event id=terminal-child|725|complete; issue/pr=#725 / PR #757; child task=child-725; parent task=parent-725; branch=eunsoogi/725-collapse-review-ceremony; worktree=/worktree; head=head; clean/index=clean; last proof=focused validator; current gate=parent review; preserved reservation/artifacts=worktree reserved; parent next action=inspect the PR; delivery=confirmed; task surface=codex task/thread\nGoal tool call: update_goal(status=\"complete\")\n"
    )
}

#[test]
fn lifecycle_audit_requires_explicit_terminal_fields() -> TestResult {
    let record = r#"{"reviewed_head":"head","profile":"strict","reviewer":{"name":"codexy-sentinel","model":"gpt-6-astra","reasoning_effort":"xhigh"},"state":"passed"}"#;
    let output = crate::support::validator_child_lane_ownership(&child_audit_evidence(record))?;
    assert!(!output.status.success(), "terminal records without direct evidence must block");
    Ok(())
}

#[test]
fn lifecycle_audit_recognizes_exact_pass_and_block() -> TestResult {
    for result in ["PASS", "BLOCK"] {
        let record = format!(r#"{{"issue_number":725,"reviewed_head":"head","profile":"strict","reviewer":{{"name":"codexy-sentinel","model":"gpt-6-astra","reasoning_effort":"xhigh"}},"terminal_result":"{result}","unresolved_findings":[],"full_review_count":1,"delta_review_count":0,"terminal_review_count":1,"terminal_review_limit":3,"terminal_review_history":[{{"id":"strict-full-1","kind":"full","reviewer":{{"name":"codexy-sentinel","model":"gpt-6-astra","reasoning_effort":"xhigh"}},"reviewed_head":"head","terminal_result":"{result}","unresolved_findings":[]}}]}}"#);
        let output = crate::support::validator_child_lane_ownership(&child_audit_evidence(&record))?;
        assert!(output.status.success(), "exact {result} must be a typed terminal: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

#[test]
fn lifecycle_audit_rejects_lowercase_terminal_results() -> TestResult {
    let record = r#"{"issue_number":725,"reviewed_head":"head","profile":"strict","reviewer":{"name":"codexy-sentinel","model":"gpt-6-astra","reasoning_effort":"xhigh"},"terminal_result":"pass","unresolved_findings":[],"full_review_count":1,"delta_review_count":0,"terminal_review_count":1,"terminal_review_limit":3,"terminal_review_history":[{"id":"strict-full-1","kind":"full","reviewer":{"name":"codexy-sentinel","model":"gpt-6-astra","reasoning_effort":"xhigh"},"reviewed_head":"head","terminal_result":"pass","unresolved_findings":[]}] }"#;
    let output = crate::support::validator_child_lane_ownership(&child_audit_evidence(record))?;
    assert!(!output.status.success(), "lowercase terminal result must not be typed evidence");
    Ok(())
}
