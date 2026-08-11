use std::process::Output;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[path = "validator_child_goal_blocked_audit/event_window.rs"]
mod event_window;
#[path = "validator_child_goal_blocked_audit/nonterminal_order.rs"]
mod nonterminal_order;
#[path = "validator_child_goal_blocked_audit/user_decision_gate.rs"]
mod user_decision_gate;

const CLASSIFICATION: &str = "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\nLane type: implementation\nSecondary surfaces: validators\nOwner decision: affirmative child-owned because the delegated child owns implementation\nAtomic scope: issue-sized\nRequired skills: orchestration\nRequired tools/evidence: goal, plan\nFirst allowed action: validate goal reports\nStop/blocker: None\n";

#[test]
fn validator_rejects_rapid_unchanged_continuations_and_crossed_parent_direction() -> TestResult {
    for (audit, pre_mutation, expected) in [
        (
            "Blocked goal audit: audit id=audit-417; first monotonic ms=1000; observed monotonic ms=1002; minimum interval ms=60000; observation ids=turn-1|turn-2|turn-3; state fingerprints=wait-a|wait-a|wait-a; producer state=sentinel-running; safe action=unavailable; wake route=sentinel-event\n".to_owned(),
            "Blocked goal pre-mutation check: audit id=audit-417; pre-delivery parent direction version=direction-1; current parent direction version=direction-1; cancellation=absent\n",
            "typed unanswered user-decision gate",
        ),
        (
            valid_gate().to_owned(),
            "Blocked goal pre-mutation check: gate id=audit-417; pre-delivery parent direction version=direction-1; current parent direction version=direction-2; cancellation=received\n",
            "cancelled by newer parent direction",
        ),
    ] {
        let output = run_validator(&blocked_evidence(audit, pre_mutation))?;
        assert!(
            !output.status.success(),
            "invalid blocked evidence unexpectedly passed"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "missing {expected:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_keeps_live_external_producers_nonterminal() -> TestResult {
    for producer in [
        "sentinel-running",
        "child-pending",
        "ci-queued",
        "connector-review-pending",
    ] {
        let output = run_validator(&blocked_evidence(
            format!("Blocked goal audit: audit id=audit-417; producer state={producer}; safe action=unavailable; wake route=unavailable\n"),
            valid_pre_mutation(),
        ))?;
        assert!(!output.status.success(), "active producer {producer} passed");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("typed unanswered user-decision gate")
        );
    }
    Ok(())
}

#[test]
fn validator_accepts_typed_user_decision_and_nonterminal_wait_handoff() -> TestResult {
    let blocked = run_validator(&blocked_evidence(
        valid_gate(),
        valid_pre_mutation(),
    ))?;
    assert!(
        blocked.status.success(),
        "typed genuine impasse should pass: {}",
        String::from_utf8_lossy(&blocked.stderr)
    );

    let waiting = run_validator(&format!(
        "{CLASSIFICATION}Nonterminal wait handoff: state fingerprint=sentinel-417-running; producer state=sentinel-running; wake route=sentinel-event; ownership=retained; goal state=active; plan state=active; goal transition=none; return control=confirmed\n"
    ))?;
    assert!(
        waiting.status.success(),
        "nonterminal wait handoff should pass: {}",
        String::from_utf8_lossy(&waiting.stderr)
    );
    Ok(())
}

#[test]
fn validator_scopes_gate_to_the_current_lane_and_detects_list_calls() -> TestResult {
    let prior_gate = format!(
        "{CLASSIFICATION}{}Lane ownership: parent-owned\n",
        valid_gate()
    );
    let current_without_gate = blocked_evidence("", valid_pre_mutation());
    for evidence in [
        format!("{prior_gate}{current_without_gate}"),
        current_without_gate.replace(
            "Goal tool call: update_goal(blocked)",
            "1. Goal tool call: update_goal(blocked)",
        ),
    ] {
        let output = run_validator(&evidence)?;
        assert!(!output.status.success(), "unscoped or hidden blocked call passed");
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("typed unanswered user-decision gate")
        );
    }
    Ok(())
}

#[test]
fn validator_binds_pre_mutation_check_to_delivered_parent_version() -> TestResult {
    let output = run_validator(&blocked_evidence(
        valid_gate(),
        "Blocked goal pre-mutation check: gate id=audit-417; pre-delivery parent direction version=direction-2; current parent direction version=direction-2; cancellation=absent\n",
    ))?;
    assert!(!output.status.success(), "unbound parent direction version passed");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("pre-delivery parent direction version")
    );
    Ok(())
}

#[test]
fn validator_requires_a_complete_nonterminal_wait_handoff() -> TestResult {
    for handoff in [
        "Nonterminal wait handoff: state fingerprint=sentinel-running; producer state=sentinel-running; wake route=sentinel-event; ownership=retained; goal transition=none; return control=confirmed",
        "Nonterminal wait handoff: state fingerprint=sentinel-running; producer state=sentinel-running; wake route=sentinel-event; ownership=retained; goal state=active; goal transition=none; return control=confirmed",
        "Nonterminal wait handoff: producer state=sentinel-running; wake route=sentinel-event; ownership=retained; goal transition=none; return control=confirmed",
        "Nonterminal wait handoff: state fingerprint=sentinel-running; producer state=none; wake route=sentinel-event; ownership=retained; goal transition=none; return control=confirmed",
        "Nonterminal wait handoff: state fingerprint=sentinel-running; producer state=sentinel-running; wake route=unavailable; ownership=retained; goal transition=none; return control=confirmed",
    ] {
        let output = run_validator(&format!("{CLASSIFICATION}{handoff}\n"))?;
        assert!(!output.status.success(), "incomplete wait handoff passed: {handoff}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("nonterminal wait handoff"));
    }
    Ok(())
}

#[test]
fn validator_normalizes_every_blocked_call_and_distinct_identifier() -> TestResult {
    let status_form = blocked_evidence(valid_gate(), valid_pre_mutation())
        .replace("update_goal(blocked)", "update_goal(status=\"blocked\")");
    let output = run_validator(&status_form)?;
    assert!(
        output.status.success(),
        "valid status-form blocked event failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    for evidence in [
        blocked_evidence("", "").replace(
            "Goal tool call: update_goal(blocked)",
            "1. - Goal tool call: update_goal(status=\"blocked\")",
        ),
        blocked_evidence(
            "Blocked goal user-decision gate: gate id=audit-417; blocker class=user-decision; decision owner=user; user question=Choose?; user response=unanswered; decision branches=same|same; material impact=changes output; safe default=unavailable; in-scope action=unavailable\n",
            valid_pre_mutation(),
        ),
        blocked_evidence(
            "Blocked goal user-decision gate: gate id=audit-417; gate id=audit-duplicate; blocker class=user-decision; decision owner=user; user question=Choose?; user response=unanswered; decision branches=one|two; material impact=changes output; safe default=unavailable; in-scope action=unavailable\n",
            valid_pre_mutation(),
        ),
    ] {
        let output = run_validator(&evidence)?;
        assert!(!output.status.success(), "malformed blocked event passed");
    }
    Ok(())
}

#[test]
fn validator_invalidates_a_check_followed_by_parent_direction() -> TestResult {
    let pre_mutation = format!(
        "{}Parent direction event: version=direction-2; cancellation=received\n",
        valid_pre_mutation()
    );
    let output = run_validator(&blocked_evidence(valid_gate(), &pre_mutation))?;
    assert!(!output.status.success(), "post-check parent correction passed");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("cancelled by newer parent direction")
    );
    Ok(())
}

#[test]
fn validator_invalidates_every_crossed_parent_direction_window() -> TestResult {
    event_window::assert_boundaries()
}

#[test]
fn validator_orders_terminal_goal_calls_after_nonterminal_waits() -> TestResult {
    nonterminal_order::assert_boundaries()
}

#[test]
fn validator_limits_blocked_goals_to_unanswered_user_decisions() -> TestResult {
    user_decision_gate::assert_boundaries()
}

fn valid_gate() -> &'static str {
    "Blocked goal user-decision gate: gate id=audit-417; blocker class=user-decision; decision owner=user; user question=Should the irreversible migration preserve legacy identifiers or replace them?; user response=unanswered; decision branches=preserve identifiers and retain compatibility|replace identifiers and require migration; material impact=the choice changes persisted identifiers and migration behavior; safe default=unavailable; in-scope action=unavailable\n"
}

fn valid_pre_mutation() -> &'static str {
    "Blocked goal pre-mutation check: gate id=audit-417; pre-delivery parent direction version=direction-1; current parent direction version=direction-1; cancellation=absent\n"
}

fn blocked_evidence(gate: impl AsRef<str>, pre_mutation: &str) -> String {
    format!(
        "{CLASSIFICATION}Source thread id: parent-417\nGoal control state: source_thread_id=parent-417\nGoal transition key: 417:blocked:audit-417\n{}Parent goal pre-delivery: operation=update_goal(blocked); parent task=parent-417; delivery=confirmed; task surface=codex task/thread; issue=#417; plan step=user-decision-gate; branch=codexy/417; worktree=/worktree; head=abc; clean/index=clean; evidence=typed user-decision gate; next action=block goal; parent direction version=direction-1; transition key=417:blocked:audit-417\nTerminal parent handoff: event id=terminal-child|417|blocked; issue/pr=#417; child task=child-417; parent task=parent-417; branch=codexy/417; worktree=/worktree; head=abc; clean/index=clean; last proof=typed user-decision gate; current gate=unanswered user decision; preserved reservation/artifacts=worktree reserved; parent next action=collect the user decision; delivery=confirmed; task surface=codex task/thread\n{pre_mutation}Goal tool call: update_goal(blocked)\nParent goal post-result: operation=update_goal(blocked); exact tool result=blocked; parent task=parent-417; delivery=confirmed; task surface=codex task/thread; transition key=417:blocked:audit-417\n",
        gate.as_ref()
    )
}

fn run_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    std::fs::write(&path, evidence)?;
    crate::support::validator_child_lane_ownership_file(&path)
}
