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
    assert_controls(
        "unrelated table remains neutral",
        &format!("{complete}\n| Check | Status |\n| --- | --- |\n| Result | pass |"),
        true,
    )?;
    Ok(())
}

#[test]
fn validator_invalidates_replacement_header_without_separator() -> TestResult {
    let evidence = format!(
        "{}\n| Field | Value |\n| Lane type | review response |\nPlan tool call: update_plan",
        classification("| --- | --- |", true)
    );
    assert_evidence("replacement header without separator", &evidence, false)
}

#[test]
fn validator_gates_setup_actions_by_authoritative_owner() -> TestResult {
    let cases = [
        (
            "delegated child owner",
            "parent-supplied",
            "child-owned",
            "affirmative child-owned because the delegated child owns implementation",
            true,
        ),
        (
            "classified child owner",
            "current-thread-classified",
            "child-owned",
            "affirmative child-owned because the classified child owns implementation",
            true,
        ),
        (
            "current thread owner",
            "current-thread-classified",
            "current-thread-owned",
            "affirmative current-thread-owned because the active thread owns implementation",
            true,
        ),
        (
            "parent owner",
            "current-thread-classified",
            "parent-owned",
            "affirmative parent-owned because the parent owns orchestration",
            false,
        ),
        (
            "external owner",
            "current-thread-classified",
            "external/human-owned",
            "affirmative external/human-owned because a maintainer owns the next action",
            false,
        ),
        (
            "invalid owner",
            "current-thread-classified",
            "unknown",
            "affirmative child-owned because the owner is not valid",
            false,
        ),
    ];
    for (name, source, owner, decision, setup_expected) in cases {
        let classification = classification_for(source, owner, decision, "| --- | --- |", true);
        assert_controls(name, &classification, setup_expected)?;
        if owner != "unknown" {
            assert_evidence(
                "goal and plan controls do not imply child setup",
                &format!("{classification}\n{}", goal_plan_controls()),
                true,
            )?;
        }
    }
    Ok(())
}

#[test]
fn validator_preserves_complete_classification_until_table_grammar_is_known() -> TestResult {
    let complete = classification("| --- | --- |", true);
    for header in [
        "| Required evidence | Status |",
        "| Required tools/evidence | Status |",
        "| Stop/blocker | Status |",
    ] {
        assert_controls(
            "schema-key-first evidence table remains neutral",
            &format!("{complete}\n{header}\n| --- | --- |\n| Result | pass |"),
            true,
        )?;
    }
    for duplicate in [
        "| Required evidence | replacement |",
        "| Stop/blocker | None |",
    ] {
        assert_controls(
            "duplicate classification row invalidates",
            &format!("{complete}\n{duplicate}"),
            false,
        )?;
    }
    Ok(())
}

fn assert_controls(name: &str, evidence: &str, expected: bool) -> TestResult {
    assert_evidence(name, &format!("{evidence}\n{}", controls()), expected)
}

fn assert_evidence(name: &str, evidence: &str, expected: bool) -> TestResult {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    std::fs::write(&path, format!("{evidence}\n"))?;
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
    classification_for(
        "parent-supplied",
        "child-owned",
        "affirmative child-owned because the delegated child owns implementation",
        delimiter,
        complete,
    )
}

fn table(delimiter: &str, complete: bool) -> String {
    table_for(
        delimiter,
        complete,
        "affirmative child-owned because the delegated child owns implementation",
    )
}

fn classification_for(
    source: &str,
    owner: &str,
    decision: &str,
    delimiter: &str,
    complete: bool,
) -> String {
    format!(
        "Ownership metadata source: {source}\nLane ownership: {owner}\nTask classification:\n{}",
        table_for(delimiter, complete, decision)
    )
}

fn table_for(delimiter: &str, complete: bool, decision: &str) -> String {
    let owner_row = format!("| Owner decision | {decision} |");
    let mut rows = vec![
        "| Lane type | implementation |",
        "| Secondary surfaces | validators |",
        owner_row.as_str(),
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

fn goal_plan_controls() -> &'static str {
    controls()
        .strip_suffix("\nChild branch codexy/463 was created after classification.")
        .expect("controls fixture must end with child setup")
}
