use std::process::Output;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const COMPLETE_CHILD_CLASSIFICATION: &str = "Task classification:\nLane type: implementation\nSecondary surfaces: validators\nOwner decision: current-thread-owned child implementation lane\nAtomic scope: issue-sized\nRequired skills: task-classification\nRequired tools/evidence: goal, plan\nFirst allowed action: validate goal reports\nStop/blocker: None\n";

#[test]
fn validator_requires_confirmed_parent_reports_for_delegated_goal_operations() -> TestResult {
    let passing = run_validator(&format!(
        "Lane ownership: child-owned\n{COMPLETE_CHILD_CLASSIFICATION}Source thread id: 019f49da-d44c-7e41-afde-8b1f7c58efa0
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
Terminal parent handoff: event id=terminal-child|375|complete; issue/pr=#375 / PR #376; child task=child-375; parent task=019f49da-d44c-7e41-afde-8b1f7c58efa0; branch=codexy/375-parent-goal-transition-reporting; worktree=/worktree; head=abc123; clean/index=clean; last proof=proof bundle; current gate=parent review; preserved reservation/artifacts=worktree reserved; parent next action=inspect the PR; delivery=confirmed; task surface=codex task/thread
Goal tool call: update_goal(complete)
Parent goal post-result: operation=update_goal(complete); exact tool result=complete; parent task=019f49da-d44c-7e41-afde-8b1f7c58efa0; delivery=confirmed; task surface=codex task/thread; transition key=375:complete:proof-bundle
Goal transition key: 375:blocked:external-impasse
Parent goal pre-delivery: operation=update_goal(blocked); parent task=019f49da-d44c-7e41-afde-8b1f7c58efa0; delivery=confirmed; task surface=codex task/thread; issue=#375; plan step=await-parent; branch=codexy/375-parent-goal-transition-reporting; worktree=/worktree; HEAD=abc123; clean/index=clean; evidence=parent delivery; next action=block goal; transition key=375:blocked:external-impasse
Terminal parent handoff: event id=terminal-child|375|blocked; issue/pr=#375 / PR #376; child task=child-375; parent task=019f49da-d44c-7e41-afde-8b1f7c58efa0; branch=codexy/375-parent-goal-transition-reporting; worktree=/worktree; head=abc123; clean/index=clean; last proof=parent delivery; current gate=execution impasse; preserved reservation/artifacts=worktree reserved; parent next action=inspect the impasse; delivery=confirmed; task surface=codex task/thread
Goal tool call: update_goal(blocked)
Parent goal post-result: operation=update_goal(blocked); exact tool result=blocked; parent task=019f49da-d44c-7e41-afde-8b1f7c58efa0; delivery=confirmed; task surface=codex task/thread; transition key=375:blocked:external-impasse
Representative static fixture: #350 restart audit: task CWD=/stale; canonical reserved worktree=/worktree; mismatch reported before goal continuation.
Representative static fixture: #360 blocked notice; #276 blocked notice; #311 usage-limited notice; #365 usage-limited notice.
",
    ))?;
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
            "Lane ownership: child-owned\nSource thread id: parent-375\nParent goal pre-delivery: operation=create_goal; parent task=parent-375; delivery=confirmed\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-375; delivery=confirmed\nParent route: agents.send_message('parent-task')\n",
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
        (
            "1. Lane ownership: child-owned\nSource thread id: parent-375\nGoal control state: source_thread_id=parent-375\nGoal transition key: 375:blocked:proof\nParent goal pre-delivery: operation=update_goal(blocked); parent task=parent-375; delivery=confirmed=false; task surface=codex task/thread unavailable; issue=; plan step=; branch=; worktree=; head=; clean/index=; evidence=; next action=; transition key=375:blocked:proof\nGoal tool call: update_goal(blocked)\nParent goal post-result: operation=update_goal(blocked); exact tool result=; parent task=parent-375; delivery=confirmed; task surface=codex task/thread; transition key=375:blocked:proof\n",
            "required pre-delivery fields",
        ),
        (
            "Lane ownership: child-owned\nSource thread id: parent-375\nGoal control state: source_thread_id=parent-375\nGoal transition key: 375:create_goal:x\nParent goal pre-delivery: operation=create_goal; parent task=parent-375; delivery=confirmed; task surface=codex task/thread; issue=#375; plan step=go; branch=codexy/375; worktree=/w; head=a; clean/index=clean; evidence=e; next action=n; transition key=375:create_goal:xyz\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=ok; parent task=parent-375; delivery=confirmed; task surface=codex task/thread; transition key=375:create_goal:xyz\n",
            "stable transition key",
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
    let isolated_parent = run_validator(
        "Lane ownership: child-owned\nSource thread id: first-parent\nLane ownership: parent-owned\nGoal tool call: update_goal(blocked)\n",
    )?;
    assert!(isolated_parent.status.success());
    let child_owner_decision = run_validator(
        "Lane ownership: child-owned\nOwner decision: current-thread-owned child implementation lane\nSource thread id: parent\nGoal tool call: update_goal(blocked)\n",
    )?;
    assert!(!child_owner_decision.status.success());
    let negated = run_validator(
        "Lane ownership: child-owned\nSource thread id: parent\nGoal control state: source_thread_id=parent\nGoal transition key: 375:create_goal:proof\nParent goal pre-delivery: operation=create_goal; parent task=parent; parent goal not: delivery=confirmed; not: task surface=codex task/thread; issue=#375; plan step=go; branch=b; worktree=w; head=h; clean/index=clean; evidence=e; next action=n; transition key=375:create_goal:proof\nGoal tool call: create_goal\n",
    )?;
    assert!(!negated.status.success());
    let numbered_parent = run_validator(
        "1. Lane ownership: child-owned\nSource thread id: first-parent\n2. Owner decision: parent-owned\nGoal tool call: update_goal(blocked)\n",
    )?;
    assert!(numbered_parent.status.success());
    Ok(())
}

#[test]
fn validator_applies_goal_reporting_to_delegated_owner_decisions() -> TestResult {
    for owner_decision in [
        "Owner decision: routing-only child delegation for static validator evidence",
        "Owner decision: current-thread-owned child implementation lane",
    ] {
        let output = run_validator(&format!(
            "{owner_decision}\nSource thread id: parent\nGoal tool call: update_goal(blocked)\n"
        ))?;
        assert!(
            !output.status.success(),
            "delegated owner decision must validate goal reports: {owner_decision}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("blocked goal operation precedes confirmed parent delivery")
        );
    }
    Ok(())
}

#[test]
fn validator_starts_a_child_lane_at_delegated_owner_decisions() -> TestResult {
    for owner_decision in [
        "Owner decision: routing-only child delegation for static validator evidence",
        "Owner decision: current-thread-owned child implementation lane",
    ] {
        let output = run_validator(&format!(
            "Owner decision: parent-owned coordination lane\nSource thread id: parent-owner\n{owner_decision}\nSource thread id: child-owner\nGoal tool call: update_goal(blocked)\n"
        ))?;
        assert!(
            !output.status.success(),
            "delegated owner decision after parent lane must validate goal reports: {owner_decision}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("blocked goal operation precedes confirmed parent delivery")
        );
    }
    Ok(())
}

#[test]
fn validator_accepts_source_thread_id_control_field_with_metadata() -> TestResult {
    let output = run_validator(
        "Lane ownership: child-owned\nSource thread id: parent\nGoal control state: source_thread_id=parent; goal_id=goal-375; status=active\nGoal transition key: 375:get_goal:inspection\nGoal tool call: get_goal\nParent goal post-result: operation=get_goal; exact tool result=active; parent task=parent; delivery=confirmed; task surface=codex task/thread; transition key=375:get_goal:inspection\n",
    )?;
    assert!(
        output.status.success(),
        "control-state metadata must retain the exact source_thread_id: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_requires_one_confirmed_terminal_handoff_before_terminal_child_transitions()
-> TestResult {
    let output = run_validator(
        "Lane ownership: child-owned\nSource thread id: parent-375\nGoal control state: source_thread_id=parent-375\nGoal transition key: 375:complete:proof\nParent goal pre-delivery: operation=update_goal(complete); parent task=parent-375; delivery=confirmed; task surface=codex task/thread; issue=#375; plan step=verify; branch=codexy/375; worktree=/worktree; head=abc; clean/index=clean; evidence=proof; next action=complete goal; transition key=375:complete:proof\nGoal tool call: update_goal(complete)\nParent goal post-result: operation=update_goal(complete); exact tool result=complete; parent task=parent-375; delivery=confirmed; task surface=codex task/thread; transition key=375:complete:proof\n",
    )?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(
        "terminal child transition requires exactly one confirmed terminal parent handoff"
    ));

    let passing = run_validator(
        "Lane ownership: child-owned\nSource thread id: parent-375\nTerminal parent handoff: event id=terminal-child|375|proof; issue/pr=#375 / PR #376; child task=child-375; parent task=parent-375; branch=codexy/375; worktree=/worktree; head=abc; clean/index=clean; last proof=focused validator; current gate=parent review; preserved reservation/artifacts=worktree reserved; parent next action=inspect the PR; delivery=confirmed; task surface=codex task/thread\nTerminal child transition: action=ownership release\n",
    )?;
    assert!(
        passing.status.success(),
        "confirmed handoff before ownership release should pass: {}",
        String::from_utf8_lossy(&passing.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_terminal_only_child_exits_without_a_handoff() -> TestResult {
    for action in ["archive", "ownership release", "blocked"] {
        let output = run_validator(&format!(
            "Lane ownership: child-owned\nTerminal child transition: action={action}\n"
        ))?;
        assert!(
            !output.status.success(),
            "terminal-only {action} evidence must not bypass parent-handoff validation"
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains(
            "terminal child transition requires exactly one confirmed terminal parent handoff"
        ));
    }

    let malformed = run_validator(
        "Lane ownership: child-owned\nTerminal parent handoff: event id=terminal-child|375|archive; issue/pr=#375 / PR #376; child task=child-375; branch=codexy/375; worktree=/worktree; head=abc; clean/index=clean; last proof=focused validator; current gate=parent review; preserved reservation/artifacts=worktree reserved; parent next action=inspect the PR; delivery=confirmed; task surface=codex task/thread\nTerminal child transition: action=archive\n",
    )?;
    assert!(!malformed.status.success());
    assert!(
        String::from_utf8_lossy(&malformed.stderr)
            .contains("terminal parent handoff is missing required confirmed delivery fields")
    );
    Ok(())
}

fn run_validator(evidence: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let evidence_path = temp.path().join("handoff.md");
    std::fs::write(&evidence_path, evidence)?;
    crate::support::validator_child_lane_ownership_file(&evidence_path)
}
