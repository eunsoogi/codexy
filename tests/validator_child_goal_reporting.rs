use std::process::{Command, Output};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_requires_confirmed_parent_reports_for_delegated_goal_operations() -> TestResult {
    let passing = run_validator(
        r#"Lane ownership: child-owned
Source thread id: 019f49da-d44c-7e41-afde-8b1f7c58efa0
Goal control state: source_thread_id=019f49da-d44c-7e41-afde-8b1f7c58efa0
Goal transition key: 375:create_goal:pending-objective
Parent goal pre-delivery: operation=create_goal; parent task=019f49da-d44c-7e41-afde-8b1f7c58efa0; delivery=confirmed; task surface=codex task/thread; issue=#375; plan step=implement; branch=codexy/375-parent-goal-transition-reporting; worktree=/worktree; HEAD=abc123; clean/index=clean; evidence=classification; next action=create goal; transition key=375:create_goal:pending-objective
Goal tool call: create_goal
Parent goal post-result: operation=create_goal; exact tool result=active goal id goal-375; parent task=019f49da-d44c-7e41-afde-8b1f7c58efa0; delivery=confirmed; task surface=codex task/thread; transition key=375:create_goal:pending-objective
Goal transition key: 375:get_goal:inspection
Goal tool call: get_goal
Parent goal post-result: operation=get_goal; exact tool result=active goal id goal-375; parent task=019f49da-d44c-7e41-afde-8b1f7c58efa0; delivery=confirmed; task surface=codex task/thread; transition key=375:get_goal:inspection
Goal transition key: 375:complete:proof-bundle
Parent goal pre-delivery: operation=update_goal(complete); parent task=019f49da-d44c-7e41-afde-8b1f7c58efa0; delivery=confirmed; task surface=codex task/thread; issue=#375; plan step=verify; branch=codexy/375-parent-goal-transition-reporting; worktree=/worktree; HEAD=abc123; clean/index=clean; evidence=proof bundle; next action=complete goal; transition key=375:complete:proof-bundle
Goal tool call: update_goal(complete)
Parent goal post-result: operation=update_goal(complete); exact tool result=complete; parent task=019f49da-d44c-7e41-afde-8b1f7c58efa0; delivery=confirmed; task surface=codex task/thread; transition key=375:complete:proof-bundle
Goal transition key: 375:blocked:external-impasse
Parent goal pre-delivery: operation=update_goal(blocked); parent task=019f49da-d44c-7e41-afde-8b1f7c58efa0; delivery=confirmed; task surface=codex task/thread; issue=#375; plan step=await-parent; branch=codexy/375-parent-goal-transition-reporting; worktree=/worktree; HEAD=abc123; clean/index=clean; evidence=parent delivery; next action=block goal; transition key=375:blocked:external-impasse
Goal tool call: update_goal(blocked)
Parent goal post-result: operation=update_goal(blocked); exact tool result=blocked; parent task=019f49da-d44c-7e41-afde-8b1f7c58efa0; delivery=confirmed; task surface=codex task/thread; transition key=375:blocked:external-impasse
Representative static fixture: #350 restart audit: task CWD=/stale; canonical reserved worktree=/worktree; mismatch reported before goal continuation.
Representative static fixture: #360 blocked notice; #276 blocked notice; #311 usage-limited notice; #365 usage-limited notice.
"#,
    )?;
    assert!(
        passing.status.success(),
        "complete parent-report evidence should pass\nstderr:\n{}",
        String::from_utf8_lossy(&passing.stderr)
    );

    for (evidence, expectation) in [
        (
            "Lane ownership: child-owned\nSource thread id: /root\n",
            "source_thread_id must name a Codex task id",
        ),
        (
            "Lane ownership: child-owned\nSource thread id: parent-375\nGoal tool call: create_goal\n",
            "pre-delivery",
        ),
        (
            "Lane ownership: child-owned\nSource thread id: parent-375\nGoal control state: source_thread_id=parent-375\nGoal transition key: 375:create_goal:objective\nParent goal pre-delivery: operation=create_goal; parent task=parent-375; delivery=confirmed\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-375; delivery=confirmed\n",
            "required pre-delivery fields",
        ),
        (
            "Lane ownership: child-owned\nSource thread id: parent-375\nGoal control state: source_thread_id=parent-375\nGoal transition key: 375:create_goal:objective\nParent goal pre-delivery: operation=create_goal; parent task=parent-375; delivery=confirmed; task surface=codex task/thread; issue=#375; plan step=implement; branch=codexy/375; worktree=/worktree; HEAD=abc; clean/index=clean; evidence=classification; next action=create; transition key=wrong\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-375; delivery=confirmed; task surface=codex task/thread; transition key=wrong\n",
            "stable transition key",
        ),
        (
            "Lane ownership: child-owned\nSource thread id: parent-375\nGoal control state: source_thread_id=parent-375\nGoal transition key: 375:create_goal:pre\nParent goal pre-delivery: operation=create_goal; parent task=parent-375; delivery=confirmed; task surface=codex task/thread; issue=#375; plan step=implement; branch=codexy/375; worktree=/worktree; HEAD=abc; clean/index=clean; evidence=classification; next action=create; transition key=375:create_goal:pre\nGoal transition key: 375:create_goal:call\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-375; delivery=confirmed; task surface=codex task/thread; transition key=375:create_goal:call\n",
            "pre-delivery receipt does not match",
        ),
        (
            "Lane ownership: child-owned\nSource thread id: parent-375\nGoal control state: source_thread_id=parent-375\nGoal transition key: 375:complete:proof\nGoal tool call: update_goal(complete)\nParent goal pre-delivery: operation=update_goal(complete); parent task=parent-375; delivery=confirmed; issue=#375; plan step=verify; branch=codexy/375; worktree=/worktree; HEAD=abc; clean/index=clean; evidence=proof; next action=complete\nParent goal post-result: operation=update_goal(complete); exact tool result=complete; parent task=parent-375; delivery=confirmed\n",
            "complete goal operation precedes confirmed parent delivery",
        ),
        (
            "Lane ownership: child-owned\nSource thread id: parent-375\nGoal control state: source_thread_id=parent-375\nGoal transition key: 375:create_goal:objective\nParent goal pre-delivery: operation=create_goal; parent task=other-parent; delivery=confirmed; issue=#375; plan step=implement; branch=codexy/375; worktree=/worktree; HEAD=abc; clean/index=clean; evidence=classification; next action=create\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=other-parent; delivery=confirmed\n",
            "wrong parent task id",
        ),
        (
            "Lane ownership: child-owned\nSource thread id: parent-375\nParent goal pre-delivery: operation=update_goal(blocked); parent task=parent-375; delivery=unavailable\nGoal tool call: update_goal(blocked)\n",
            "blocked goal operation precedes confirmed parent delivery",
        ),
        (
            "Lane ownership: child-owned\nSource thread id: parent-375\nGoal tool call: get_goal\n",
            "post-result",
        ),
        (
            "Lane ownership: child-owned\nSource thread id: parent-375\nGoal control state: source_thread_id=parent-375\nGoal transition key: 375:get_goal:inspection\nGoal tool call: get_goal\nParent goal post-result: operation=get_goal; result was active; parent task=parent-375; delivery=confirmed; task surface=codex task/thread; transition key=375:get_goal:inspection\n",
            "prose-only",
        ),
        (
            "Lane ownership: child-owned\nSource thread id: parent-375\nParent goal pre-delivery: operation=create_goal; parent task=parent-375; delivery=confirmed\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-375; delivery=confirmed\nParent route: agents.send_message('/root')\n",
            "local agents",
        ),
        (
            "Lane ownership: child-owned\nSource thread id: parent-375\nRestart audit: task CWD=/stale; canonical reserved worktree=/canonical\nGoal tool call: get_goal\nParent goal post-result: operation=get_goal; exact tool result=active; parent task=parent-375; delivery=confirmed\n",
            "worktree mismatch",
        ),
        (
            "Lane ownership: child-owned\nSource thread id: parent-375\nGoal control state: source_thread_id=parent-375\nGoal transition key: 375:create_goal:objective\nParent goal pre-delivery: operation=create_goal; parent task=parent-375; delivery=confirmed; issue=#375; plan step=implement; branch=codexy/375; worktree=/worktree; HEAD=abc; clean/index=clean; evidence=classification; next action=create\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-375; delivery=confirmed\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-375; delivery=confirmed\n",
            "duplicate goal call",
        ),
    ] {
        let output = run_validator(evidence)?;
        assert!(
            !output.status.success(),
            "validator should reject {expectation}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expectation),
            "missing {expectation} diagnostic: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
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
