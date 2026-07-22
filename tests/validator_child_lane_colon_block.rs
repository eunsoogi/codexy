type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_bounds_colon_classification_to_its_declared_block() -> TestResult {
    for classification in [complete_colon(), complete_gfm()] {
        for alias in [
            "Required tools/evidence: handoff receipt",
            "Required tools: handoff receipt",
            "Required evidence: handoff receipt",
        ] {
            assert_result(
                "complete classification ignores later alias metadata",
                &format!("{classification}\n{alias}\n{}", controls()),
                true,
            )?;
        }
    }
    for invalid in [
        complete_colon().replacen(
            "Secondary surfaces: validators",
            "Lane type: duplicate\nSecondary surfaces: validators",
            1,
        ),
        complete_colon().replacen(
            "Lane type: implementation\nSecondary surfaces: validators",
            "Secondary surfaces: validators\nLane type: implementation",
            1,
        ),
        format!("{}\nTask classification:\nLane type: incomplete", complete_colon()),
        format!(
            "{}\n| Field | Value |\n| --- | --- |\n| Lane type | incomplete |",
            complete_colon()
        ),
    ] {
        assert_result(
            "active or replacement classification defects fail closed",
            &format!("{invalid}\n{}", controls()),
            false,
        )?;
    }
    for neutral in [
        "Workflow evidence: captured",
        "| Check | Status |\n| --- | --- |\n| Rust | pass |",
    ] {
        assert_result(
            "unrelated metadata and tables remain neutral",
            &format!("{}\n{neutral}\n{}", complete_colon(), controls()),
            true,
        )?;
    }
    assert_result(
        "a complete explicit replacement is valid",
        &format!("{}\n{}\n{}", complete_colon(), complete_colon(), controls()),
        true,
    )
}

fn assert_result(name: &str, evidence: &str, expected: bool) -> TestResult {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    std::fs::write(&path, evidence)?;
    let output = crate::support::validator_child_lane_ownership_file(&path)?;
    assert_eq!(
        output.status.success(),
        expected,
        "{name}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn complete_colon() -> &'static str {
    "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\nLane type: implementation\nSecondary surfaces: validators\nOwner decision: affirmative child-owned because the delegated child owns implementation\nAtomic scope: issue-sized\nRequired skills: task-classification\nRequired tools/evidence: goal, plan\nFirst allowed action: implement after classification\nStop/blocker: None"
}

fn complete_gfm() -> &'static str {
    "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\n| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | affirmative child-owned because the child owns implementation |\n| Atomic scope | issue-sized |\n| Required skills | task-classification |\n| Required tools/evidence | goal, plan |\n| First allowed action | implement after classification |\n| Stop/blocker | None |"
}

fn controls() -> &'static str {
    "Source thread id: parent-463\nGoal control state: source_thread_id=parent-463\nGoal transition key: 463:create_goal:classification\nParent goal pre-delivery: operation=create_goal; parent task=parent-463; delivery=confirmed; task surface=codex task/thread; issue=#463; plan step=implement; branch=codexy/463; worktree=/worktree; head=abc; clean/index=clean; evidence=classification; next action=create goal; transition key=463:create_goal:classification\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-463; delivery=confirmed; task surface=codex task/thread; transition key=463:create_goal:classification\nPlan tool call: update_plan\nChild branch codexy/463 was created after classification."
}
