type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_distinguishes_invalid_replacement_delimiters_from_absent_tables() -> TestResult {
    let complete = classification("| --- | --- |", true);
    for delimiter in [
        "| -- | --- |",
        "| --- | |",
        "| --- |",
        "| --- | --- | --- |",
        "| --- | --- | extra |",
        r"| ---\|--- | --- |",
        "| --- | --x |",
    ] {
        assert_controls(
            "malformed repeated delimiter invalidates",
            &format!("{complete}\n{}", table(delimiter, true)),
            false,
        )?;
    }
    for delimiter in ["| --- | --- |", "| :--- | ---: |", "| ---: | :--- |"] {
        assert_controls(
            "valid repeated delimiter replaces",
            &format!("{complete}\n{}", table(delimiter, true)),
            true,
        )?;
    }
    assert_controls(
        "valid repeated incomplete table invalidates",
        &format!("{complete}\n{}", table("| --- | --- |", false)),
        false,
    )?;
    for suffix in [
        "| Field | Value |",
        "| Check | Status |\n| --- | --- |\n| Result | pass |",
        "| Field | Value |\nRequired evidence: tests",
    ] {
        assert_controls(
            "header-only or unrelated table remains neutral",
            &format!("{complete}\n{suffix}"),
            true,
        )?;
    }
    Ok(())
}

fn assert_controls(name: &str, evidence: &str, expected: bool) -> TestResult {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    std::fs::write(&path, format!("{evidence}\n{}\n", controls()))?;
    let output = crate::support::validator_child_lane_ownership_file(&path)?;
    assert_eq!(
        output.status.success(),
        expected,
        "{name}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn classification(delimiter: &str, complete: bool) -> String {
    format!(
        "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\n{}",
        table(delimiter, complete)
    )
}

fn table(delimiter: &str, complete: bool) -> String {
    let mut rows = vec![
        "| Lane type | implementation |",
        "| Secondary surfaces | validators |",
        "| Owner decision | affirmative child-owned because the delegated child owns implementation |",
        "| Atomic scope | issue-sized |",
        "| Required skills | task-classification |",
        "| Required tools/evidence | goal, plan |",
        "| First allowed action | implement after classification |",
        "| Stop/blocker | None |",
    ];
    if !complete {
        rows.pop();
    }
    format!(
        "| Field | Value |\n{delimiter}\n{}",
        rows.join("\n")
    )
}

fn controls() -> &'static str {
    "Source thread id: parent-463\nGoal control state: source_thread_id=parent-463\nGoal transition key: 463:create_goal:delimiter\nParent goal pre-delivery: operation=create_goal; parent task=parent-463; delivery=confirmed; task surface=codex task/thread; issue=#463; plan step=implement; branch=codexy/463; worktree=/worktree; head=abc; clean/index=clean; evidence=classification; next action=create goal; transition key=463:create_goal:delimiter\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-463; delivery=confirmed; task surface=codex task/thread; transition key=463:create_goal:delimiter\nPlan tool call: update_plan\nChild branch codexy/463 was created after classification."
}
