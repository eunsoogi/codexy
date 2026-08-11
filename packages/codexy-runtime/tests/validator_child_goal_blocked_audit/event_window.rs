use super::{TestResult, blocked_evidence, run_validator, valid_gate, valid_pre_mutation};

const DIRECTION: &str = "Parent direction event: version=direction-2; cancellation=received\n";

pub(super) fn assert_boundaries() -> TestResult {
    for pre_mutation in [
        format!("{DIRECTION}{}", valid_pre_mutation()),
        format!("{}{DIRECTION}", valid_pre_mutation()),
        format!(
            "{DIRECTION}{}Parent direction event: version=direction-3; cancellation=received\n{}",
            valid_pre_mutation(),
            valid_pre_mutation(),
        ),
    ] {
        assert_rejected(&blocked_evidence(valid_gate(), &pre_mutation))?;
    }

    let stale = blocked_evidence(valid_gate(), valid_pre_mutation());
    let fresh = stale.replacen(
        "Goal tool call: update_goal(blocked)",
        &format!(
            "{DIRECTION}{}Parent goal pre-delivery: operation=update_goal(blocked); parent task=parent-417; delivery=confirmed; task surface=codex task/thread; issue=#417; plan step=user-decision-gate; branch=codexy/417; worktree=/worktree; head=abc; clean/index=clean; evidence=typed user-decision gate; next action=block goal; parent direction version=direction-3; transition key=417:blocked:audit-417\n{}Goal tool call: update_goal(blocked)",
            valid_gate(),
            "Blocked goal pre-mutation check: gate id=audit-417; pre-delivery parent direction version=direction-3; current parent direction version=direction-3; cancellation=absent\n",
        ),
        1,
    );
    let output = run_validator(&fresh)?;
    assert!(
        output.status.success(),
        "fresh audit window failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn assert_rejected(evidence: &str) -> TestResult {
    let output = run_validator(evidence)?;
    assert!(
        !output.status.success(),
        "crossed parent-direction window passed"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("cancelled by newer parent direction")
    );
    Ok(())
}
