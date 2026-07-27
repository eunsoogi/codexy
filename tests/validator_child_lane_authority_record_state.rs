type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_gates_authority_records_until_they_are_complete() -> TestResult {
    let table = classification_table();
    for record in [
        "Ownership metadata source: parent-supplied".to_owned(),
        "Ownership metadata source: parent-supplied\nLane ownership: child-owned".to_owned(),
        format!("Ownership metadata source: parent-supplied\nTask classification:\n{table}"),
        format!("Lane ownership: parent-owned\nTask classification:\n{table}"),
        format!("Ownership metadata source: parent supplied\nLane ownership: parent-owned\nTask classification:\n{}", table.replace("affirmative child-owned because the delegated child owns implementation", "affirmative parent-owned because the parent owns orchestration")),
        format!("Ownership metadata source: current-thread-classified\nLane ownership: unknown\nTask classification:\n{table}"),
        format!("Task classification:\n{table}"),
    ] {
        assert_each_action(&record, false)?;
    }
    Ok(())
}

#[test]
fn validator_scopes_complete_authority_records_to_the_current_lane() -> TestResult {
    for (source, owner, decision, expected_setup) in [
        ("current-thread-classified", "parent-owned", "affirmative parent-owned because the parent owns orchestration", false),
        ("current-thread-classified", "external/human-owned", "affirmative external/human-owned because a maintainer owns the next action", false),
        ("parent-supplied", "child-owned", "affirmative child-owned because the delegated child owns implementation", true),
        ("current-thread-classified", "current-thread-owned", "affirmative current-thread-owned because the active thread owns implementation", true),
    ] {
        let record = authority_record(source, owner, decision);
        assert_action(&record, "Goal tool call: create_goal", true)?;
        assert_action(&record, "Plan tool call: update_plan", true)?;
        assert_action(&record, "Child branch codexy/463 was created after classification.", expected_setup)?;
    }
    assert_each_action("Ownership metadata source: parent-supplied\nPR: #464", true)
}

fn assert_each_action(evidence: &str, expected: bool) -> TestResult {
    for action in ["Goal tool call: create_goal", "Plan tool call: update_plan", "Child branch codexy/463 was created after classification."] {
        assert_action(evidence, action, expected)?;
    }
    Ok(())
}

fn assert_action(evidence: &str, action: &str, expected: bool) -> TestResult {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    let action = if expected && action == "Goal tool call: create_goal" { reported_goal() } else { action };
    std::fs::write(&path, format!("{evidence}\n{action}\n"))?;
    let output = crate::support::validator_child_lane_ownership_file(&path)?;
    assert_eq!(output.status.success(), expected, "{action}: {}", String::from_utf8_lossy(&output.stderr));
    Ok(())
}

fn authority_record(source: &str, owner: &str, decision: &str) -> String {
    format!("Ownership metadata source: {source}\nLane ownership: {owner}\nTask classification:\n{}", classification_table().replace("affirmative child-owned because the delegated child owns implementation", decision))
}

fn classification_table() -> &'static str {
    "| Field | Value |\n| --- | --- |\n| Lane type | implementation |\n| Secondary surfaces | validators |\n| Owner decision | affirmative child-owned because the delegated child owns implementation |\n| Atomic scope | issue-sized |\n| Required skills | task-classification |\n| Required tools/evidence | goal, plan |\n| First allowed action | implement after classification |\n| Stop/blocker | None |"
}

fn reported_goal() -> &'static str {
    "Source thread id: parent-463\nGoal control state: source_thread_id=parent-463\nGoal transition key: 463:create_goal:authority\nParent goal pre-delivery: operation=create_goal; parent task=parent-463; delivery=confirmed; task surface=codex task/thread; issue=#463; plan step=verify; branch=codexy/463; worktree=/worktree; head=abc; clean/index=clean; evidence=authority; next action=create goal; transition key=463:create_goal:authority\nGoal tool call: create_goal\nParent goal post-result: operation=create_goal; exact tool result=active; parent task=parent-463; delivery=confirmed; task surface=codex task/thread; transition key=463:create_goal:authority"
}
