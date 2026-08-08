use std::process::Output;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const CLASSIFICATION: &str = "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\nLane type: implementation\nSecondary surfaces: validators\nOwner decision: affirmative child-owned because the delegated child owns implementation\nAtomic scope: issue-sized\nRequired skills: task-classification\nRequired tools/evidence: goal, plan\nFirst allowed action: validate goal reports\nStop/blocker: None\n";

#[test]
fn validator_rejects_rapid_unchanged_continuations_and_crossed_parent_direction() -> TestResult {
    for (audit, pre_mutation, expected) in [
        (
            "Blocked goal audit: audit id=audit-417; first monotonic ms=1000; observed monotonic ms=1002; minimum interval ms=60000; observation ids=turn-1|turn-2|turn-3; state fingerprints=wait-a|wait-a|wait-a; producer state=sentinel-running; safe action=unavailable; wake route=sentinel-event\n".to_owned(),
            "Blocked goal pre-mutation check: audit id=audit-417; pre-delivery parent direction version=direction-1; current parent direction version=direction-1; cancellation=absent\n",
            "distinct material observations",
        ),
        (
            valid_audit("none"),
            "Blocked goal pre-mutation check: audit id=audit-417; pre-delivery parent direction version=direction-1; current parent direction version=direction-2; cancellation=received\n",
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
            valid_audit(producer),
            valid_pre_mutation(),
        ))?;
        assert!(!output.status.success(), "active producer {producer} passed");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("active external producer")
        );
    }
    Ok(())
}

#[test]
fn validator_accepts_typed_genuine_impasse_and_nonterminal_wait_handoff() -> TestResult {
    let blocked = run_validator(&blocked_evidence(
        valid_audit("none"),
        valid_pre_mutation(),
    ))?;
    assert!(
        blocked.status.success(),
        "typed genuine impasse should pass: {}",
        String::from_utf8_lossy(&blocked.stderr)
    );

    let waiting = run_validator(&format!(
        "{CLASSIFICATION}Nonterminal wait handoff: state fingerprint=sentinel-417-running; producer state=sentinel-running; wake route=sentinel-event; ownership=retained; goal transition=none; return control=confirmed\n"
    ))?;
    assert!(
        waiting.status.success(),
        "nonterminal wait handoff should pass: {}",
        String::from_utf8_lossy(&waiting.stderr)
    );
    Ok(())
}

#[test]
fn validator_scopes_audit_to_the_current_lane_and_detects_list_calls() -> TestResult {
    let prior_audit = format!(
        "{CLASSIFICATION}{}Lane ownership: parent-owned\n",
        valid_audit("none")
    );
    let current_without_audit = blocked_evidence("", valid_pre_mutation());
    for evidence in [
        format!("{prior_audit}{current_without_audit}"),
        current_without_audit.replace(
            "Goal tool call: update_goal(blocked)",
            "1. Goal tool call: update_goal(blocked)",
        ),
    ] {
        let output = run_validator(&evidence)?;
        assert!(!output.status.success(), "unscoped or hidden blocked call passed");
        assert!(String::from_utf8_lossy(&output.stderr).contains("typed blocked goal audit"));
    }
    Ok(())
}

#[test]
fn validator_binds_pre_mutation_check_to_delivered_parent_version() -> TestResult {
    let output = run_validator(&blocked_evidence(
        valid_audit("none"),
        "Blocked goal pre-mutation check: audit id=audit-417; pre-delivery parent direction version=direction-2; current parent direction version=direction-2; cancellation=absent\n",
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

fn valid_audit(producer: &str) -> String {
    format!(
        "Blocked goal audit: audit id=audit-417; first monotonic ms=1000; observed monotonic ms=61000; minimum interval ms=60000; observation ids=observation-a|observation-b|observation-c; state fingerprints=state-a|state-b|state-c; producer state={producer}; safe action=unavailable; wake route=unavailable\n"
    )
}

fn valid_pre_mutation() -> &'static str {
    "Blocked goal pre-mutation check: audit id=audit-417; pre-delivery parent direction version=direction-1; current parent direction version=direction-1; cancellation=absent\n"
}

fn blocked_evidence(audit: impl AsRef<str>, pre_mutation: &str) -> String {
    format!(
        "{CLASSIFICATION}Source thread id: parent-417\nGoal control state: source_thread_id=parent-417\nGoal transition key: 417:blocked:audit-417\n{}Parent goal pre-delivery: operation=update_goal(blocked); parent task=parent-417; delivery=confirmed; task surface=codex task/thread; issue=#417; plan step=impasse-audit; branch=codexy/417; worktree=/worktree; head=abc; clean/index=clean; evidence=typed blocked audit; next action=block goal; parent direction version=direction-1; transition key=417:blocked:audit-417\nTerminal parent handoff: event id=terminal-child|417|blocked; issue/pr=#417; child task=child-417; parent task=parent-417; branch=codexy/417; worktree=/worktree; head=abc; clean/index=clean; last proof=typed blocked audit; current gate=execution impasse; preserved reservation/artifacts=worktree reserved; parent next action=inspect impasse; delivery=confirmed; task surface=codex task/thread\n{pre_mutation}Goal tool call: update_goal(blocked)\nParent goal post-result: operation=update_goal(blocked); exact tool result=blocked; parent task=parent-417; delivery=confirmed; task surface=codex task/thread; transition key=417:blocked:audit-417\n",
        audit.as_ref()
    )
}

fn run_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    std::fs::write(&path, evidence)?;
    crate::support::validator_child_lane_ownership_file(&path)
}
