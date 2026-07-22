type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_arbitrary_ninth_gfm_classification_row() -> TestResult {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    std::fs::write(
        &path,
        format!(
            "{}\n| Approval | skipped |\nPlan tool call: update_plan\n",
            classification_table()
        ),
    )?;
    let output = crate::support::validator_child_lane_ownership_file(&path)?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(
        "child-owned lane control evidence includes create_goal or update_plan before formal $task-classification evidence completed"
    ));
    Ok(())
}

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
            "later canonical incomplete results table invalidates",
            format!("{complete}\n{}", results_table()),
            false,
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
            "wrong header cannot authorize classification rows",
            complete.replacen("| Field | Value |", "| Check | Status |", 1),
            false,
        ),
        (
            "later wrong-header classification-like rows remain neutral",
            format!(
                "{complete}\n{}",
                repeated_table.replacen("| Field | Value |", "| Check | Status |", 1)
            ),
            true,
        ),
        (
            "adjacent canonical rows reordered fail closed",
            complete.replacen(
                "| Lane type | implementation |\n| Secondary surfaces | validators |",
                "| Secondary surfaces | validators |\n| Lane type | implementation |",
                1,
            ),
            false,
        ),
        (
            "duplicate canonical row fails closed",
            complete.replacen(
                "| Secondary surfaces | validators |",
                "| Lane type | implementation |",
                1,
            ),
            false,
        ),
        (
            "trailing classification row fails closed",
            format!("{complete}\n| Lane type | another implementation |"),
            false,
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
            "header-only canonical candidate invalidates",
            format!("{complete}\n| Field | Value |"),
            false,
        ),
        (
            "canonical classification-like rows replace prior classification",
            format!("{complete}\n{}", results_table()),
            false,
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
    let rows = classification_rows();
    for index in 0..rows.len() - 1 {
        let mut reordered = rows.to_vec();
        reordered.swap(index, index + 1);
        assert_result(
            "each adjacent canonical reorder fails closed",
            &classification_with_rows(&reordered, "| Field | Value |"),
            false,
        )?;
    }
    let mut reversed = rows.to_vec();
    reversed.reverse();
    assert_result(
        "wider canonical permutation fails closed",
        &classification_with_rows(&reversed, "| Field | Value |"),
        false,
    )?;
    for index in 0..rows.len() {
        let mut duplicated = rows.to_vec();
        duplicated.insert(index, rows[index]);
        assert_result(
            "each duplicate canonical field fails closed",
            &classification_with_rows(&duplicated, "| Field | Value |"),
            false,
        )?;
        let mut omitted = rows.to_vec();
        omitted.remove(index);
        assert_result(
            "each omitted canonical field fails closed",
            &classification_with_rows(&omitted, "| Field | Value |"),
            false,
        )?;
    }
    for header in [
        "| Check | Status |",
        "| Fields | Value |",
        "| Field | Values |",
    ] {
        assert_result(
            "wrong header cannot authorize canonical-looking rows",
            &classification_with_rows(&rows, header),
            false,
        )?;
    }
    assert_result(
        "malformed canonical delimiter cannot authorize",
        &classification_table().replacen("| --- | --- |", "| -- | --- |", 1),
        false,
    )?;
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

fn classification_table() -> String {
    classification_with_rows(&classification_rows(), "| Field | Value |")
}

fn classification_with_rows(rows: &[&str], header: &str) -> String {
    format!(
        "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\n{header}\n| --- | --- |\n{}",
        rows.join("\n")
    )
}

fn classification_rows() -> [&'static str; 8] {
    [
        "| Lane type | implementation |",
        "| Secondary surfaces | validators |",
        "| Owner decision | affirmative child-owned because the delegated child owns implementation |",
        "| Atomic scope | issue-sized |",
        "| Required skills | task-classification |",
        "| Required tools/evidence | goal, plan |",
        "| First allowed action | implement after classification |",
        "| Stop/blocker | None |",
    ]
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
