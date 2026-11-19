//! Requires one delivered parent handoff before terminal child exits.

use std::process::Output;

#[path = "validator_child_terminal_handoff/records.rs"]
mod records;
#[path = "validator_child_terminal_handoff/readable_table.rs"]
mod readable_table;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn native_goal_operations_do_not_require_plugin_receipts() -> TestResult {
    // Native goal operations are governed by the host, independently of child handoff receipts.
    let create = run_validator("Lane ownership: child-owned\nGoal tool call: create_goal\n")?;
    assert!(create.status.success(), "{}", String::from_utf8_lossy(&create.stderr));
    for status in ["complete", "blocked"] {
        let evidence = format!(
            "Lane ownership: child-owned\nGoal tool call: update_goal(status=\"{status}\")\n"
        );
        let output = run_validator(&evidence)?;
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

fn validator_rejects_local_parent_tasks_in_terminal_only_handoffs() -> TestResult {
    for parent_task in ["/root", "agents.send_message('/root')", "codex task/thread"] {
        let output = run_validator(&terminal_only_evidence(parent_task))?;
        assert!(
            !output.status.success(),
            "terminal-only handoff must reject local parent task {parent_task:?}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("terminal parent handoff is missing required confirmed delivery fields")
        );
    }

    let valid = run_validator(&terminal_only_evidence(
        "019f49da-d44c-7e41-afde-8b1f7c58efa0",
    ))?;
    assert!(
        valid.status.success(),
        "Codex task id must remain valid: {}",
        String::from_utf8_lossy(&valid.stderr)
    );
    Ok(())
}

fn validator_normalizes_bullet_terminal_records() -> TestResult {
    let missing_handoff = run_validator(
        "Lane ownership: child-owned\n- Terminal child transition: action=archive\n",
    )?;
    assert!(
        !missing_handoff.status.success(),
        "a bullet terminal transition must require a handoff"
    );

    let bullet = run_validator(&format!(
        "Lane ownership: child-owned\n- {}- Terminal child transition: action=archive\n",
        terminal_handoff("archive:bullet")
    ))?;
    assert!(
        bullet.status.success(),
        "matching bullet records must remain valid: {}",
        String::from_utf8_lossy(&bullet.stderr)
    );
    Ok(())
}

fn validator_rejects_placeholder_or_local_child_tasks_in_terminal_handoffs() -> TestResult {
    for child_task in ["codex task/thread", "child task", "/root", "agents.worker"] {
        let output = run_validator(&terminal_only_evidence_for_tasks("parent-375", child_task))?;
        assert!(
            !output.status.success(),
            "terminal handoff must reject placeholder child task {child_task:?}"
        );
    }

    let valid = run_validator(&terminal_only_evidence_for_tasks("parent-375", "child-375"))?;
    assert!(
        valid.status.success(),
        "concrete child task id must remain valid: {}",
        String::from_utf8_lossy(&valid.stderr)
    );
    Ok(())
}

fn validator_requires_handoff_for_suffixed_terminal_actions() -> TestResult {
    let output = run_validator(
        "Lane ownership: child-owned\nTerminal child transition: action=archive; reason=done\n",
    )?;
    assert!(
        !output.status.success(),
        "suffixed terminal action must still require a parent handoff"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains(
        "terminal child transition requires exactly one confirmed terminal parent handoff"
    ));
    Ok(())
}

fn validator_rejects_duplicate_handoffs_before_one_terminal_transition() -> TestResult {
    let handoff = terminal_handoff("archive:first");
    let duplicate = run_validator(&format!(
        "Lane ownership: child-owned\n{handoff}{handoff}Terminal child transition: action=archive\n"
    ))?;
    assert!(
        !duplicate.status.success(),
        "a terminal transition must reject duplicate confirmed handoffs"
    );
    assert!(
        String::from_utf8_lossy(&duplicate.stderr)
            .contains("terminal parent handoff must not be repeated before terminal transition")
    );

    let single = run_validator(&format!(
        "Lane ownership: child-owned\n{handoff}Terminal child transition: action=archive\n"
    ))?;
    assert!(
        single.status.success(),
        "exactly one handoff must remain valid: {}",
        String::from_utf8_lossy(&single.stderr)
    );
    Ok(())
}

fn validator_keeps_one_handoff_for_related_terminal_transitions() -> TestResult {
    let output = run_validator(
        "Lane ownership: child-owned\nTerminal parent handoff: event id=terminal-child|375|complete; issue/pr=#375 / PR #376; child task=child-375; parent task=parent-375; branch=codexy/375; worktree=/worktree; head=abc; clean/index=clean; last proof=focused validator; current gate=parent review; preserved reservation/artifacts=worktree reserved; parent next action=inspect the PR; delivery=confirmed; task surface=codex task/thread\nTerminal child transition: action=stop\nTerminal child transition: action=ownership release\n",
    )?;
    assert!(output.status.success());
    Ok(())
}

fn validator_rejects_handoff_after_stop_before_ownership_release() -> TestResult {
    let handoff = terminal_handoff("complete:stop");
    let duplicate = run_validator(&format!(
        "Lane ownership: child-owned\n{handoff}Terminal child transition: action=stop\n{handoff}Terminal child transition: action=ownership release\n"
    ))?;
    assert!(
        !duplicate.status.success(),
        "a same-exit ownership release must reject a second handoff after stop"
    );
    assert!(
        String::from_utf8_lossy(&duplicate.stderr)
            .contains("terminal parent handoff must not be repeated before terminal transition")
    );
    Ok(())
}

fn terminal_only_evidence(parent_task: &str) -> String {
    terminal_only_evidence_for_tasks(parent_task, "child-375")
}

fn terminal_only_evidence_for_tasks(parent_task: &str, child_task: &str) -> String {
    format!(
        "Lane ownership: child-owned\n{}Terminal child transition: action=archive\n",
        records::handoff_for_tasks("archive", parent_task, child_task)
    )
}

fn terminal_handoff(event: &str) -> String {
    records::handoff_for_parent(event, "parent-375")
}

fn run_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    crate::support::validator_child_lane_ownership(evidence)
}

#[test]
fn validator_child_terminal_handoff_matrix() -> TestResult {
    validator_rejects_local_parent_tasks_in_terminal_only_handoffs()?;
    validator_normalizes_bullet_terminal_records()?;
    validator_rejects_placeholder_or_local_child_tasks_in_terminal_handoffs()?;
    validator_requires_handoff_for_suffixed_terminal_actions()?;
    validator_rejects_duplicate_handoffs_before_one_terminal_transition()?;
    validator_keeps_one_handoff_for_related_terminal_transitions()?;
    validator_rejects_handoff_after_stop_before_ownership_release()?;
    Ok(())
}
