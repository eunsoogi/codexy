type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_handles_utf8_and_pipe_parity_without_panicking() -> TestResult {
    for (name, row, expected) in [
        ("multibyte final character", "| 설명", false),
        ("multibyte content with closing pipe", "| Secondary surfaces | 검증 |", true),
        ("unicode before escaped pipe", r"| Secondary surfaces | 검증\|문서 |", true),
        ("unicode before unescaped pipe", "| Secondary surfaces | 검증|문서 |", false),
        ("odd backslash parity", r"| Secondary surfaces | 검증\\\|문서 |", true),
        ("even backslash parity", r"| Secondary surfaces | 검증\\|문서 |", false),
        ("leading pipe only", "|", false),
        ("empty row", "||", false),
        ("canonical two-cell row", "| Secondary surfaces | validators |", true),
        ("ascii without trailing pipe", "| Secondary surfaces | validators", false),
    ] {
        assert_row(name, row, expected)?;
    }
    Ok(())
}

#[test]
fn validator_fails_closed_for_malformed_utf8_table_delimiters() -> TestResult {
    for delimiter in [
        "| --- | 설명",
        "| 설명 | --- |",
        "| --- | -- |",
        r"| --- \| --- |",
    ] {
        let evidence = format!(
            "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\n| Field | Value |\n{delimiter}\n{}\n{}",
            classification_rows("| Secondary surfaces | validators |"),
            controls()
        );
        assert_validation("malformed delimiter", &evidence, false)?;
    }
    Ok(())
}

fn assert_row(name: &str, row: &str, expected: bool) -> TestResult {
    let evidence = format!(
        "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\n| Field | Value |\n| --- | --- |\n{}\n{}",
        classification_rows(row),
        controls()
    );
    assert_validation(name, &evidence, expected)
}

fn assert_validation(name: &str, evidence: &str, expected: bool) -> TestResult {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    std::fs::write(&path, format!("{evidence}\n"))?;
    let output = crate::support::validator_child_lane_ownership_file(&path)?;
    assert_ne!(
        output.status.code(),
        Some(101),
        "{name} panicked: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.status.success(),
        expected,
        "{name}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn classification_rows(secondary_row: &str) -> String {
    [
        "| Lane type | implementation |",
        secondary_row,
        "| Owner decision | affirmative child-owned because the delegated child owns implementation |",
        "| Atomic scope | issue-sized |",
        "| Required skills | task-classification |",
        "| Required tools/evidence | goal, plan |",
        "| First allowed action | implement after classification |",
        "| Stop/blocker | None |",
    ]
    .join("\n")
}

fn controls() -> &'static str {
    "Source thread id: parent-463\nGoal control state: source_thread_id=parent-463\nGoal transition key: 463:create_goal:utf8\nParent goal pre-delivery: operation=create_goal; parent task=parent-463; delivery=confirmed; task surface=codex task/thread; issue=#463; plan step=implement; branch=codexy/463; worktree=/worktree; head=abc; clean/index=clean; evidence=classification; next action=create goal; transition key=463:create_goal:utf8\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-463; delivery=confirmed; task surface=codex task/thread; transition key=463:create_goal:utf8\nPlan tool call: update_plan\nChild branch codexy/463 was created after classification."
}
