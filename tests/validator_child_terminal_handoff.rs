use std::process::{Command, Output};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_local_parent_tasks_in_terminal_only_handoffs() -> TestResult {
    for parent_task in ["/root", "agents.send_message('/root')"] {
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

#[test]
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

#[test]
fn validator_requires_handoff_for_status_form_goal_transitions() -> TestResult {
    for status in ["complete", "blocked"] {
        let missing = run_validator(&format!(
            "Lane ownership: child-owned\nGoal tool call: update_goal(status=\"{status}\")\n"
        ))?;
        assert!(
            !missing.status.success(),
            "status-form {status} transition must require a handoff"
        );
    }

    let accepted = run_validator(
        "Lane ownership: child-owned\nSource thread id: parent-375\nGoal control state: source_thread_id=parent-375\nGoal transition key: 375:complete:proof\nParent goal pre-delivery: operation=update_goal(status=\"complete\"); parent task=parent-375; delivery=confirmed; task surface=codex task/thread; issue=#375; plan step=verify; branch=codexy/375; worktree=/worktree; head=abc; clean/index=clean; evidence=proof; next action=complete; transition key=375:complete:proof\nTerminal parent handoff: event id=terminal-child|375|complete; issue/pr=#375 / PR #376; child task=child-375; parent task=parent-375; branch=codexy/375; worktree=/worktree; head=abc; clean/index=clean; last proof=focused validator; current gate=parent review; preserved reservation/artifacts=worktree reserved; parent next action=inspect the PR; delivery=confirmed; task surface=codex task/thread\nGoal tool call: update_goal(status=\"complete\")\nParent goal post-result: operation=update_goal(status=\"complete\"); exact tool result=complete; parent task=parent-375; delivery=confirmed; task surface=codex task/thread; transition key=375:complete:proof\n",
    )?;
    assert!(accepted.status.success());
    Ok(())
}

fn terminal_only_evidence(parent_task: &str) -> String {
    format!(
        "Lane ownership: child-owned\nTerminal parent handoff: event id=terminal-child|375|archive; issue/pr=#375 / PR #376; child task=child-375; parent task={parent_task}; branch=codexy/375; worktree=/worktree; head=abc; clean/index=clean; last proof=focused validator; current gate=parent review; preserved reservation/artifacts=worktree reserved; parent next action=inspect the PR; delivery=confirmed; task surface=codex task/thread\nTerminal child transition: action=archive\n"
    )
}

fn run_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-child-lane-ownership", "--evidence-file"])
        .arg(&evidence_path)
        .output()?)
}
