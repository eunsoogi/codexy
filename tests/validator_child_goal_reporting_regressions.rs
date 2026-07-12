use std::process::{Command, Output};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_checks_numbered_child_goal_metadata() -> TestResult {
    let output = run_validator(
        "1. Lane ownership: child-owned\n2. Source thread id: parent-375\n3. Goal control state: source_thread_id=parent-375\n4. Goal transition key: opaque:receipt:key\n5. Goal tool call: update_goal(blocked)\n",
    )?;

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("blocked goal operation precedes confirmed parent delivery")
    );
    Ok(())
}

#[test]
fn validator_accepts_opaque_transition_keys_and_negated_local_agent_policy() -> TestResult {
    let output = run_validator(
        "Lane ownership: child-owned\nSource thread id: parent-375\nGoal control state: source_thread_id=parent-375\nGoal transition key: opaque:receipt:key\nParent goal pre-delivery: operation=create_goal; parent task=parent-375; delivery=confirmed; task surface=codex task/thread; issue=#375; plan step=verify; branch=codexy/375; worktree=/worktree; head=abc; clean/index=clean; evidence=proof; next action=create goal; transition key=opaque:receipt:key\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-375; delivery=confirmed; task surface=codex task/thread; transition key=opaque:receipt:key\nCompliance: MUST NOT use agents.send_message('/root') for parent delivery.\n",
    )?;

    assert!(
        output.status.success(),
        "opaque keys and a negated local-agent policy must pass: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_local_agent_routes_beside_a_negated_policy() -> TestResult {
    let output = run_validator(
        "Lane ownership: child-owned\nSource thread id: parent-375\nGoal control state: source_thread_id=parent-375\nGoal transition key: opaque:receipt:key\nParent goal pre-delivery: operation=create_goal; parent task=parent-375; delivery=confirmed; task surface=codex task/thread; issue=#375; plan step=verify; branch=codexy/375; worktree=/worktree; head=abc; clean/index=clean; evidence=proof; next action=create goal; transition key=opaque:receipt:key\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-375; delivery=confirmed; task surface=codex task/thread; transition key=opaque:receipt:key\nParent route: agents.send_message('parent-task'), policy: MUST NOT use agents.send_message('/root').\n",
    )?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("local agents"));
    Ok(())
}

#[test]
fn validator_rejects_receipt_values_that_look_like_owner_headers() -> TestResult {
    let output = run_validator(
        "Lane ownership: child-owned\nSource thread id: parent-375\nGoal control state: source_thread_id=parent-375\nGoal transition key: opaque:receipt:key\nParent goal pre-delivery: operation=create_goal; parent task=parent-375; delivery=confirmed; task surface=codex task/thread; issue=#375; plan step=verify; branch=codexy/375; worktree=/worktree; head=abc; clean/index=clean; evidence=owner decision: current-thread-owned child implementation lane; next action=create goal; transition key=opaque:receipt:key\nGoal tool call: create_goal\n",
    )?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("post-result"));
    Ok(())
}

#[test]
fn validator_keeps_bullet_parent_owner_headers_outside_child_lanes() -> TestResult {
    let output = run_validator(
        "Lane ownership: child-owned\nSource thread id: child-375\nGoal control state: source_thread_id=child-375\nGoal transition key: opaque:receipt:key\nParent goal pre-delivery: operation=create_goal; parent task=child-375; delivery=confirmed; task surface=codex task/thread; issue=#375; plan step=verify; branch=codexy/375; worktree=/worktree; head=abc; clean/index=clean; evidence=proof; next action=create goal; transition key=opaque:receipt:key\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=child-375; delivery=confirmed; task surface=codex task/thread; transition key=opaque:receipt:key\n- Owner decision: parent-owned orchestration lane\nGoal tool call: update_goal(blocked)\n",
    )?;

    assert!(
        output.status.success(),
        "a normalized parent owner header must end the preceding child lane: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_local_agent_routes_after_negated_policy_text() -> TestResult {
    let output = run_validator(
        "Lane ownership: child-owned\nSource thread id: parent-375\nGoal control state: source_thread_id=parent-375\nGoal transition key: opaque:receipt:key\nParent goal pre-delivery: operation=create_goal; parent task=parent-375; delivery=confirmed; task surface=codex task/thread; issue=#375; plan step=verify; branch=codexy/375; worktree=/worktree; head=abc; clean/index=clean; evidence=proof; next action=create goal; transition key=opaque:receipt:key\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-375; delivery=confirmed; task surface=codex task/thread; transition key=opaque:receipt:key\nCompliance: MUST NOT use agents.send_message('/root'); actual route: agents.send_message('parent-task').\n",
    )?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("local agents"));
    Ok(())
}

#[test]
fn validator_accepts_multiple_routes_named_by_one_negated_policy() -> TestResult {
    let output = run_validator(
        "Lane ownership: child-owned\nSource thread id: parent-375\nGoal control state: source_thread_id=parent-375\nGoal transition key: opaque:receipt:key\nParent goal pre-delivery: operation=create_goal; parent task=parent-375; delivery=confirmed; task surface=codex task/thread; issue=#375; plan step=verify; branch=codexy/375; worktree=/worktree; head=abc; clean/index=clean; evidence=proof; next action=create goal; transition key=opaque:receipt:key\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-375; delivery=confirmed; task surface=codex task/thread; transition key=opaque:receipt:key\nCompliance: MUST NOT use agents.send_message('/root') or agents.send_message('parent-task').\n",
    )?;

    assert!(
        output.status.success(),
        "one negated policy must cover each named local-agent route: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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
