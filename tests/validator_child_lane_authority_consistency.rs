type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_requires_display_owner_to_match_governing_authority() -> TestResult {
    let cases = [
        (
            "current-thread exact metadata with explicit affirmation",
            classification(
                "current-thread-classified",
                "current-thread-owned",
                "affirmative current-thread-owned because parent-owned is reserved for orchestration",
            ),
            true,
        ),
        (
            "child exact metadata with explicit affirmation",
            classification(
                "current-thread-classified",
                "child-owned",
                "affirmative child-owned because the delegated child owns implementation",
            ),
            true,
        ),
        (
            "current authority with child display conflict",
            classification(
                "current-thread-classified",
                "current-thread-owned",
                "affirmative child-owned because the delegated child owns implementation",
            ),
            false,
        ),
        (
            "child authority with current display conflict",
            classification(
                "current-thread-classified",
                "child-owned",
                "affirmative current-thread-owned because the active thread owns issue-sized work",
            ),
            false,
        ),
        (
            "parent authority is not a child lane",
            classification(
                "current-thread-classified",
                "parent-owned",
                "affirmative current-thread-owned because the active thread owns issue-sized work",
            ),
            false,
        ),
        (
            "unknown authority token",
            classification(
                "current-thread-classified",
                "unknown",
                "affirmative current-thread-owned because the active thread owns issue-sized work",
            ),
            false,
        ),
        (
            "missing authority metadata",
            classification_without_metadata(
                "affirmative current-thread-owned because the active thread owns issue-sized work",
            ),
            false,
        ),
        (
            "malformed display owner token",
            classification(
                "current-thread-classified",
                "current-thread-owned",
                "affirmative current-thread-ownedness because the active thread owns issue-sized work",
            ),
            false,
        ),
        (
            "missing display owner row",
            classification_without_owner("current-thread-classified", "current-thread-owned"),
            false,
        ),
        (
            "multiple authoritative selections fail closed",
            classification(
                "current-thread-classified",
                "child-owned or parent-owned",
                "affirmative child-owned because the delegated child owns implementation",
            ),
            false,
        ),
        (
            "conjoined authoritative selections fail closed",
            classification(
                "current-thread-classified",
                "child-owned and parent-owned",
                "affirmative child-owned because the delegated child owns implementation",
            ),
            false,
        ),
        (
            "authoritative prose suffix fails closed",
            classification(
                "current-thread-classified",
                "child-owned for implementation",
                "affirmative child-owned because the delegated child owns implementation",
            ),
            false,
        ),
        (
            "denied display decision fails closed",
            classification(
                "current-thread-classified",
                "current-thread-owned",
                "denied current-thread-owned because not allowed to act",
            ),
            false,
        ),
        (
            "unmarked display rationale fails closed",
            classification(
                "current-thread-classified",
                "current-thread-owned",
                "current-thread-owned because not allowed to act",
            ),
            false,
        ),
        (
            "ambiguous display selection fails closed",
            classification(
                "current-thread-classified",
                "current-thread-owned",
                "affirmative current-thread-owned or child-owned",
            ),
            false,
        ),
    ];
    for (name, evidence, expected) in cases {
        assert_owner_result(name, &evidence, expected)?;
    }
    assert_owner_result(
        "latest repeated table governs",
        &format!(
            "{}\n{}",
            classification(
                "current-thread-classified",
                "current-thread-owned",
                "affirmative current-thread-owned because the active thread owns issue-sized work",
            ),
            classification_table("affirmative child-owned because the delegated child owns implementation"),
        ),
        false,
    )?;
    assert_owner_result(
        "repeated complete matching table remains valid",
        &format!(
            "{}\n{}",
            classification(
                "current-thread-classified",
                "current-thread-owned",
                "affirmative current-thread-owned because the active thread owns issue-sized work",
            ),
            classification_table("affirmative current-thread-owned because unknown ownership is elsewhere"),
        ),
        true,
    )
}

fn assert_owner_result(name: &str, evidence: &str, expected: bool) -> TestResult {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    std::fs::write(&path, format!("{evidence}\n{}\n", controls()))?;
    let output = crate::support::validator_child_lane_ownership_file(&path)?;
    assert_eq!(
        output.status.success(),
        expected,
        "{name}: goal/plan/branch controls\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn classification(source: &str, owner: &str, display_owner: &str) -> String {
    format!(
        "Ownership metadata source: {source}\nLane ownership: {owner}\nTask classification:\n{}",
        classification_table(display_owner)
    )
}

fn classification_without_metadata(display_owner: &str) -> String {
    format!("Task classification:\n{}", classification_table(display_owner))
}

fn classification_without_owner(source: &str, owner: &str) -> String {
    format!(
        "Ownership metadata source: {source}\nLane ownership: {owner}\nTask classification:\n{}",
        classification_table("").replace("| Owner decision |  |\n", "")
    )
}

fn classification_table(display_owner: &str) -> String {
    format!(
        "| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | {display_owner} |\n| Atomic scope | issue-sized |\n| Required skills | task-classification |\n| Required tools/evidence | goal, plan |\n| First allowed action | implement after classification |\n| Stop/blocker | None |"
    )
}

fn controls() -> &'static str {
    "Source thread id: parent-463\nGoal control state: source_thread_id=parent-463\nGoal transition key: 463:create_goal:classification\nParent goal pre-delivery: operation=create_goal; parent task=parent-463; delivery=confirmed; task surface=codex task/thread; issue=#463; plan step=implement; branch=codexy/463; worktree=/worktree; head=abc; clean/index=clean; evidence=classification; next action=create goal; transition key=463:create_goal:classification\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-463; delivery=confirmed; task surface=codex task/thread; transition key=463:create_goal:classification\nPlan tool call: update_plan\nChild branch codexy/463 was created after classification."
}
