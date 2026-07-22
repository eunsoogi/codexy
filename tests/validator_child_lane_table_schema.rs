type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_uses_only_recognized_gfm_schemas_to_replace_classification() -> TestResult {
    let complete = classification_table();
    let repeated_table = complete
        .split_once("Task classification:\n")
        .expect("fixture has a classification marker")
        .1;
    let incomplete = repeated_table.replacen("| Stop/blocker | None |", "", 1);
    let cases = [
        (
            "unrelated two-column results remain neutral",
            format!("{complete}\n{}", results_table()),
            true,
        ),
        (
            "differently headed results remain neutral",
            format!("{complete}\n{}", checks_table()),
            true,
        ),
        (
            "arbitrary nonclassification headers remain neutral",
            format!("{complete}\n{}", audit_table()),
            true,
        ),
        (
            "later recognized incomplete table invalidates",
            format!("{complete}\n{incomplete}"),
            false,
        ),
        (
            "later recognized complete table replaces",
            format!("{complete}\n{repeated_table}"),
            true,
        ),
        (
            "unrelated table before classification cannot authorize",
            complete.replacen(
                "Task classification:\n",
                &format!("{}\n{}\nTask classification:\n", results_table(), controls()),
                1,
            ),
            false,
        ),
        (
            "header-only candidate remains neutral",
            format!("{complete}\n| Field | Value |"),
            true,
        ),
        (
            "nonclassification table rows remain neutral",
            format!("{complete}\n{}", results_table()),
            true,
        ),
        (
            "mixed candidate fails closed",
            format!(
                "{complete}\n| Field | Value |\n| --- | --- |\n| Result | pass |\n| Lane type | implementation |"
            ),
            false,
        ),
        (
            "malformed recognized candidate fails closed",
            format!(
                "{complete}\n| Field | Value |\n| --- | --- |\n| Lane type | implementation | extra |"
            ),
            false,
        ),
    ];
    for (name, classification, expected) in cases {
        assert_result(name, &classification, expected)?;
    }
    Ok(())
}

fn assert_result(name: &str, classification: &str, expected: bool) -> TestResult {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    std::fs::write(&path, format!("{classification}\n{}\n", controls()))?;
    let output = crate::support::validator_child_lane_ownership_file(&path)?;
    assert_eq!(
        output.status.success(),
        expected,
        "{name}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn classification_table() -> &'static str {
    "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\n| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | affirmative child-owned because the delegated child owns implementation |\n| Atomic scope | issue-sized |\n| Required skills | task-classification |\n| Required tools/evidence | goal, plan |\n| First allowed action | implement after classification |\n| Stop/blocker | None |"
}

fn results_table() -> &'static str {
    "| Field | Value |\n| --- | --- |\n| Result | pass |\n| Evidence | captured |"
}

fn checks_table() -> &'static str {
    "| Check | Status |\n| --- | --- |\n| Rust tests | pass |\n| Evidence | captured |"
}

fn audit_table() -> &'static str {
    "| Artifact | Outcome |\n| --- | --- |\n| Config | valid |\n| CI | green |"
}

fn controls() -> &'static str {
    "Source thread id: parent-463\nGoal control state: source_thread_id=parent-463\nGoal transition key: 463:create_goal:classification\nParent goal pre-delivery: operation=create_goal; parent task=parent-463; delivery=confirmed; task surface=codex task/thread; issue=#463; plan step=implement; branch=codexy/463; worktree=/worktree; head=abc; clean/index=clean; evidence=classification; next action=create goal; transition key=463:create_goal:classification\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-463; delivery=confirmed; task surface=codex task/thread; transition key=463:create_goal:classification\nPlan tool call: update_plan\nChild branch codexy/463 was created after classification."
}
